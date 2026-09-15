#![forbid(unsafe_code)]

//! Story 16-2 / AC3+AC4 — fake-host REPL vectors (D-16-2-B/C/D).
//!
//! The `ShellHost` port keeps the REPL testable with no kernel objects: the
//! halt arm, the refusals, the resolution renders, the proceeding turn and
//! the EOF behaviour are driven against a host the test scripts, through the
//! same `run_shell` production calls.
//!
//! The reader is GATED (a `BufRead` impl that withholds EOF until the test
//! releases it): a `Cursor` hits EOF at once and EOF ends the session, so
//! the door-path and context-race vectors — a live REPL with NO stdin line —
//! would be unprovable without it.

use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::sync::Arc;

use maos_domain::halt::HaltState;
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::capability::CapError;
use maos_domain::ports::inference::{
    InferenceError, InferencePort, InferenceRequest, InferenceResponse, ProviderAttribution,
    StopReason, TokenUsage,
};
use maos_shell::{ResolveRefusal, ShellHost};
use parking_lot::{Condvar, Mutex};

// ─────────────────────────────────────────────────────────────────────────────
// The gated reader — EOF withheld until the test releases it
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Gate {
    bytes: std::collections::VecDeque<u8>,
    eof: bool,
}

/// A `BufRead` over a test-held gate: `send_line` appends, `release_eof`
/// ends the stream. `read` parks on a condvar, so the REPL's reader thread
/// blocks WITHOUT spinning while the test drives the host.
struct GatedInput {
    gate: Mutex<Gate>,
    ready: Condvar,
}

impl GatedInput {
    fn new() -> Self {
        Self {
            gate: Mutex::new(Gate::default()),
            ready: Condvar::new(),
        }
    }

    fn send_line(&self, line: &str) {
        let mut g = self.gate.lock();
        for b in line.as_bytes() {
            g.bytes.push_back(*b);
        }
        g.bytes.push_back(b'\n');
        self.ready.notify_all();
    }

    fn release_eof(&self) {
        let mut g = self.gate.lock();
        g.eof = true;
        self.ready.notify_all();
    }
}

impl GatedInput {
    /// Block until at least one byte is available (or EOF), then hand the
    /// currently buffered bytes out as an owned Vec — the `BufRead` loan in
    /// [`GatedInputReader`] is served from that copy.
    fn take_bytes_blocking(&self) -> Vec<u8> {
        let mut g = self.gate.lock();
        loop {
            if !g.bytes.is_empty() {
                return g.bytes.drain(..).collect();
            }
            if g.eof {
                return Vec::new();
            }
            self.ready
                .wait_for(&mut g, std::time::Duration::from_millis(10));
        }
    }
}

/// A `Write` into a shared buffer the test can read while the REPL runs.
#[derive(Clone, Default)]
struct SharedOutput(Arc<Mutex<Vec<u8>>>);

impl Write for SharedOutput {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SharedOutput {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock()).into_owned()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The scripted fake host
// ─────────────────────────────────────────────────────────────────────────────

/// Everything the REPL asked its host to do.
#[derive(Default)]
struct HostCalls {
    raised: Vec<(String, String)>,
    recorded_turns: Vec<String>,
    resolutions: Vec<(String, String)>,
}

type ResolveScript = Box<dyn Fn(&str) -> Result<(), ResolveRefusal> + Send + Sync>;

struct FakeHost {
    pid: u32,
    calls: Arc<Mutex<HostCalls>>,
    raise_err: Option<String>,
    /// halt id → current state; the TEST mutates this mid-run (the door's
    /// effect on the registry).
    states: Arc<Mutex<HashMap<String, HaltState>>>,
    /// halt id → context; `None` until the test writes it (the resolver's
    /// memory write, which can land AFTER the state flip — Trap 5).
    contexts: Arc<Mutex<HashMap<String, String>>>,
    resolve: ResolveScript,
}

impl FakeHost {
    fn ok(pid: u32) -> (Self, HostHandle) {
        Self::scripted(pid, Box::new(|_| Ok(())))
    }

    fn scripted(pid: u32, resolve: ResolveScript) -> (Self, HostHandle) {
        let calls = Arc::new(Mutex::new(HostCalls::default()));
        let states = Arc::new(Mutex::new(HashMap::new()));
        let contexts = Arc::new(Mutex::new(HashMap::new()));
        (
            Self {
                pid,
                calls: Arc::clone(&calls),
                raise_err: None,
                states: Arc::clone(&states),
                contexts: Arc::clone(&contexts),
                resolve,
            },
            HostHandle {
                pid,
                calls,
                states,
                contexts,
            },
        )
    }

    fn with_raise_err(pid: u32, err: String) -> (Self, HostHandle) {
        let (mut host, handle) = Self::ok(pid);
        host.raise_err = Some(err);
        (host, handle)
    }
}

/// The test-side handle into the scripted host.
#[derive(Clone)]
struct HostHandle {
    pid: u32,
    calls: Arc<Mutex<HostCalls>>,
    states: Arc<Mutex<HashMap<String, HaltState>>>,
    contexts: Arc<Mutex<HashMap<String, String>>>,
}

impl HostHandle {
    fn mint_id(&self, n: usize) -> String {
        next_ulid(n)
    }

    fn set_state(&self, id: &str, state: HaltState) {
        self.states.lock().insert(id.to_string(), state);
    }

    fn deliver_context(&self, id: &str, text: &str) {
        self.contexts
            .lock()
            .insert(id.to_string(), text.to_string());
    }

    fn snapshot(&self) -> HostCalls {
        self.calls.lock().clone()
    }
}

impl Clone for HostCalls {
    fn clone(&self) -> Self {
        Self {
            raised: self.raised.clone(),
            recorded_turns: self.recorded_turns.clone(),
            resolutions: self.resolutions.clone(),
        }
    }
}

fn next_ulid(n: usize) -> String {
    // Deterministic, ULID-shaped (26 Crockford base32 chars).
    let alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut s = String::new();
    let mut n = n as u64 + 1;
    for i in 0..26 {
        if i == 0 {
            s.push(alphabet.chars().nth((n % 8) as usize).unwrap());
        } else {
            n = n.wrapping_mul(1103515245).wrapping_add(12345);
            s.push(alphabet.chars().nth((n % 32) as usize).unwrap());
        }
    }
    s
}

impl ShellHost for FakeHost {
    fn spirit_pid(&self) -> u32 {
        self.pid
    }

    fn issue_turn_token(&self) -> Result<CapabilityToken, CapError> {
        Ok(CapabilityToken::new(
            TokenId([1; 16]),
            self.pid,
            u64::MAX,
            [0u8; 64],
        ))
    }

    fn record_turn(&self, _token: &CapabilityToken, payload: &[u8]) -> Result<(), CapError> {
        self.calls
            .lock()
            .recorded_turns
            .push(String::from_utf8_lossy(payload).into_owned());
        Ok(())
    }

    fn raise_ambiguity_halt(
        &self,
        tag: &str,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut calls = self.calls.lock();
        let n = calls.raised.len();
        let id = next_ulid(n);
        calls.raised.push((tag.to_string(), prompt.to_string()));
        drop(calls);
        if let Some(err) = &self.raise_err {
            return Err(err.to_string().into());
        }
        self.states
            .lock()
            .insert(id.clone(), HaltState::PendingResolution);
        Ok(id)
    }

    fn halt_state(&self, halt_id: &str) -> Option<HaltState> {
        self.states.lock().get(halt_id).cloned()
    }

    fn take_context(&self, halt_id: &str) -> Option<String> {
        self.contexts.lock().get(halt_id).cloned()
    }

    fn resolve_with_context(&self, halt_id: &str, text: &str) -> Result<(), ResolveRefusal> {
        self.calls
            .lock()
            .resolutions
            .push((halt_id.to_string(), text.to_string()));
        (self.resolve)(halt_id)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The recording inference port
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct RecordingInference(Arc<Mutex<Vec<String>>>);

impl InferencePort for RecordingInference {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        self.0.lock().push(req.prompt);
        Ok(InferenceResponse {
            text: "mock response".into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "mock".into(),
                endpoint_url: "http://mock".into(),
                model_id: None,
            },
        })
    }
}

/// Run a live REPL: the gated input withholds EOF, the output is shared, the
/// REPL runs on its own thread. Returns the joiner; the caller drives the
/// scene through the gates and reads `out`/`handle` while it runs.
struct LiveRepl {
    input: Arc<GatedInput>,
    out: SharedOutput,
    handle: HostHandle,
    prompts: Arc<Mutex<Vec<String>>>,
    done: Arc<std::sync::atomic::AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl LiveRepl {
    fn start(host: FakeHost, handle: HostHandle, prompts: Arc<Mutex<Vec<String>>>) -> Self {
        let input = Arc::new(GatedInput::new());
        let out = SharedOutput::default();
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let reader = GatedInputReader {
            inner: Arc::clone(&input),
            loaned: Vec::new(),
        };
        let writer = out.clone();
        let done_flag = Arc::clone(&done);
        let prompts_for_repl = Arc::clone(&prompts);
        let join = std::thread::spawn(move || {
            let result = maos_shell::run_shell(
                Arc::new(RecordingInference(prompts_for_repl)),
                &host,
                reader,
                writer,
                maos_cli::accessibility::ColorChoice::Never,
                "anthropic",
                "/tmp/t/transparency.sqlite",
            );
            done_flag.store(result.is_ok(), std::sync::atomic::Ordering::SeqCst);
        });
        Self {
            input,
            out,
            handle,
            prompts,
            done,
            join: Some(join),
        }
    }

    fn wait_for(&self, needle: &str, label: &str) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            if self.out.text().contains(needle) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!(
            "{label}: `{needle}` never appeared; output:\n{}",
            self.out.text()
        );
    }
}

impl Drop for LiveRepl {
    fn drop(&mut self) {
        self.input.release_eof();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// `impl BufRead + Send + 'static` over the shared gate: the `BufRead` loan
/// is served from an owned copy taken (blocking) from the gate.
struct GatedInputReader {
    inner: Arc<GatedInput>,
    loaned: Vec<u8>,
}

impl BufRead for GatedInputReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.loaned.is_empty() {
            self.loaned = self.inner.take_bytes_blocking();
        }
        Ok(&self.loaned)
    }

    fn consume(&mut self, amt: usize) {
        self.loaned.drain(..amt);
    }
}

impl Read for GatedInputReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let source = self.fill_buf()?;
        let n = source.len().min(buf.len());
        buf[..n].copy_from_slice(&source[..n]);
        self.consume(n);
        Ok(n)
    }
}

impl RecordingInference {
    fn clone_from(prompts: &Arc<Mutex<Vec<String>>>) -> Self {
        Self(Arc::clone(prompts))
    }
}

/// Drive `run_shell` synchronously with a Cursor (EOF immediately after the
/// given lines) — for the vectors that need no live tick.
fn run_sync(host: &FakeHost, input: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut output = Vec::new();
    let result = maos_shell::run_shell(
        Arc::new(RecordingInference::default()),
        host,
        std::io::Cursor::new(input.as_bytes().to_vec()),
        &mut output,
        maos_cli::accessibility::ColorChoice::Never,
        "anthropic",
        "/tmp/t/transparency.sqlite",
    );
    result.map(|()| output)
}

const AMBIGUOUS: &str = "@hello-spirit refactor src/main.rs to be more idiomatic";

/// Raise a halt on the live REPL and return the minted id once it renders.
fn raise_live(repl: &LiveRepl) -> String {
    repl.input.send_line(AMBIGUOUS);
    repl.wait_for("[HALT task.acceptance_criterion.ambiguous]", "halt line");
    let id = {
        let calls = repl.handle.snapshot();
        assert_eq!(calls.raised.len(), 1, "exactly one raise");
        next_ulid(0)
    };
    id
}

// ─────────────────────────────────────────────────────────────────────────────
// AC3 — the halt arm
// ─────────────────────────────────────────────────────────────────────────────

/// The `[HALT …]` line and its `halt <id>` follow-up print ONLY after
/// `raise_ambiguity_halt` returned `Ok`, with the id the host minted.
#[test]
fn halt_line_prints_only_after_the_kernel_accepted_the_halt() {
    let (host, handle) = FakeHost::ok(1);
    let output = run_sync(&host, &format!("{AMBIGUOUS}\n")).expect("shell completes");
    let text = String::from_utf8(output).unwrap();
    assert!(
        text.contains("[HALT task.acceptance_criterion.ambiguous]"),
        "the [HALT tag] line must render; output:\n{text}"
    );
    let minted = handle.mint_id(0);
    assert!(
        text.contains(&format!("halt {minted} — type a clarification")),
        "the halt line names the minted id; output:\n{text}"
    );
    assert!(
        text.contains("maosctl halt resolve"),
        "the halt line shows the door alternative; output:\n{text}"
    );
    let calls = handle.snapshot();
    assert!(
        calls.recorded_turns.is_empty(),
        "a halted turn records no shell.turn row"
    );
}

/// A host whose raise returns `Err` prints `maos: error: …`, no `[HALT`
/// line, and the shell KEEPS SERVING the next line.
#[test]
fn a_failed_raise_is_an_error_line_and_the_shell_keeps_serving() {
    let (host, _handle) =
        FakeHost::with_raise_err(1, "registry insert failed: duplicate".to_string());
    let output = run_sync(
        &host,
        "@hello-spirit make it better\n@hello-spirit say hi\n",
    )
    .expect("shell completes on both lines");
    let text = String::from_utf8(output).unwrap();
    assert!(
        !text.contains("[HALT"),
        "no [HALT line may print when the kernel refused; output:\n{text}"
    );
    assert!(
        text.contains("maos: error: registry insert failed"),
        "the refusal renders as an error line; output:\n{text}"
    );
    assert!(
        text.contains("mock response"),
        "the next line is served after the error; output:\n{text}"
    );
}

/// Every REPL line flows through the output writer (D-16-2-B seam).
#[test]
fn every_repl_line_flows_through_the_output_writer() {
    let (host, _handle) = FakeHost::ok(1);
    let output = run_sync(&host, "@hello-spirit say hi\n").unwrap();
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("maos shell — type @hello-spirit <msg>"));
    assert!(text.contains("mock response"));
    assert!(text.contains("maos: shell exiting"));
}

/// The butler prototype arm is unchanged (AC2).
#[test]
fn butler_pick_prototype_arm_is_unchanged() {
    let (host, handle) = FakeHost::ok(1);
    let output = run_sync(&host, "@butler pick a\n").unwrap();
    let text = String::from_utf8(output).unwrap();
    assert!(
        text.contains("Linear note written"),
        "butler arm intact: {text}"
    );
    let calls = handle.snapshot();
    assert!(calls.raised.is_empty() && calls.recorded_turns.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// AC4 — two surfaces, one mechanism; the Spirit proceeds (D-16-2-C/D)
// ─────────────────────────────────────────────────────────────────────────────

/// The REPL-submitted clarification: resolution renders on the TICK (from the
/// registry), then the Spirit proceeds with the context — the J0 beat.
#[test]
fn typed_clarification_resolves_then_the_spirit_proceeds_with_the_context() {
    let (host, handle) = FakeHost::ok(1);
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let repl = LiveRepl::start(host, handle.clone(), Arc::clone(&prompts));
    let id = raise_live(&repl);

    repl.input.send_line("idiomatic = clippy-clean, no unwrap");
    // The scripted resolve is Ok; the registry flips only when the test does
    // it (the door's effect):
    repl.handle.set_state(&id, HaltState::Resumed);
    repl.handle
        .deliver_context(&id, "idiomatic = clippy-clean, no unwrap");

    repl.wait_for(
        &format!("halt {id} resolved: provided_context"),
        "resolution",
    );
    repl.wait_for("mock response", "proceeding turn");
    {
        let calls = handle.snapshot();
        assert_eq!(
            calls.resolutions.len(),
            1,
            "the clarification was submitted through the door surface"
        );
        assert_eq!(
            calls.resolutions[0].1,
            "idiomatic = clippy-clean, no unwrap"
        );
        assert_eq!(
            calls.recorded_turns.len(),
            1,
            "the proceeding turn recorded shell.turn"
        );
    }
    let prompts = prompts.lock();
    assert_eq!(
        prompts.len(),
        1,
        "exactly one inference call — the proceeding turn"
    );
    assert!(
        prompts[0].contains("refactor src/main.rs"),
        "the proceeding turn re-runs the directive; prompt: {}",
        prompts[0]
    );
    assert!(
        prompts[0].contains("idiomatic = clippy-clean, no unwrap"),
        "the context is carried into the inference request; prompt: {}",
        prompts[0]
    );
    let text = repl.out.text();
    assert!(
        !text.contains("context not delivered"),
        "context was delivered; output:\n{text}"
    );
}

/// The DOOR path (AC4): a host whose state flips to `Resumed` with NO stdin
/// line — the resolution and the proceeding turn render unprompted. This is
/// the vector the render-on-submit falsifier reds.
#[test]
fn door_path_resolution_renders_unprompted_within_ticks() {
    let (host, handle) = FakeHost::ok(1);
    let repl = LiveRepl::start(host, handle, Arc::new(Mutex::new(Vec::new())));
    let id = raise_live(&repl);

    // NO stdin line. The other surface resolves it:
    repl.handle.set_state(&id, HaltState::Resumed);
    repl.handle.deliver_context(&id, "door-side clarification");

    repl.wait_for(
        &format!("halt {id} resolved: provided_context"),
        "door resolution",
    );
    repl.wait_for("mock response", "door proceeding turn");
    let calls = repl.handle.snapshot();
    assert!(
        calls.resolutions.is_empty(),
        "no REPL submission on the door path"
    );
}

/// `Terminated` — the turn is abandoned, no inference call.
#[test]
fn terminated_renders_turn_abandoned_and_dispatches_nothing() {
    let (host, handle) = FakeHost::ok(1);
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let repl = LiveRepl::start(host, handle, Arc::clone(&prompts));
    let id = raise_live(&repl);
    repl.handle.set_state(&id, HaltState::Terminated);
    repl.wait_for(&format!("halt {id} resolved: accepted_halt"), "terminated");
    repl.wait_for("turn abandoned (accepted_halt)", "abandon line");
    assert!(
        prompts.lock().is_empty(),
        "a terminated turn makes no inference call"
    );
}

/// `Overridden` — proceeds WITHOUT context.
#[test]
fn overridden_proceeds_without_context() {
    let (host, handle) = FakeHost::ok(1);
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let repl = LiveRepl::start(host, handle, Arc::clone(&prompts));
    let id = raise_live(&repl);
    repl.handle.set_state(&id, HaltState::Overridden);
    repl.wait_for(
        &format!("halt {id} resolved: authorized_override"),
        "override",
    );
    repl.wait_for("mock response", "override proceeding turn");
    let prompts = prompts.lock();
    assert_eq!(prompts.len(), 1);
    assert!(
        !prompts[0]
            .to_lowercase()
            .contains("clarification from the operator"),
        "the overridden turn proceeds without a clarification; prompt: {}",
        prompts[0]
    );
}

/// A `halt_already_resolved` refusal is printed and the winner still renders
/// (two concurrent resolutions; the loser gets the typed 409).
#[test]
fn halt_already_resolved_is_printed_and_the_winner_still_renders() {
    let (host, handle) = FakeHost::scripted(
        1,
        Box::new(|_| {
            Err(ResolveRefusal {
                code: "halt_already_resolved".to_string(),
                detail: "resolved by the door".to_string(),
            })
        }),
    );
    let repl = LiveRepl::start(host, handle, Arc::new(Mutex::new(Vec::new())));
    let id = raise_live(&repl);

    // The registry has NOT flipped yet in the REPL's view, the operator
    // types — the door refuses with the typed 409:
    repl.input.send_line("idiomatic = clippy-clean, no unwrap");
    repl.wait_for("your text was not sent", "not-sent line");
    assert!(
        repl.out.text().contains("was already resolved"),
        "the refusal line says the halt was already resolved; output: {}",
        repl.out.text()
    );

    // The winner renders on the next tick regardless:
    repl.handle.set_state(&id, HaltState::Resumed);
    repl.handle.deliver_context(&id, "door got there first");
    repl.wait_for(&format!("halt {id} resolved: provided_context"), "winner");
    repl.wait_for("mock response", "winner proceeding turn");
}

/// R8's late clarification: no `halt is pending` prefix — the line names the
/// halt and its kind, and nothing was submitted beyond the refused attempt.
#[test]
fn late_clarification_names_the_halt_and_kind() {
    let (host, handle) = FakeHost::scripted(
        1,
        Box::new(|_| {
            Err(ResolveRefusal {
                code: "halt_already_resolved".to_string(),
                detail: "halt not pending".to_string(),
            })
        }),
    );
    let repl = LiveRepl::start(host, handle, Arc::new(Mutex::new(Vec::new())));
    let id = raise_live(&repl);
    // The door already resolved it (provided_context) — the REPL's registry
    // view still shows pending, so the typed line reaches the refusal path
    // and reads the kind from the state the registry would show:
    repl.handle.set_state(&id, HaltState::PendingResolution);
    repl.input.send_line("clarification that lost the race");
    repl.wait_for("no halt is pending", "late line");
    let text = repl.out.text();
    assert!(
        text.contains(&format!("halt {id} was already resolved")),
        "the late line names the halt; output:\n{text}"
    );
}

/// While a halt is pending, `@hello-spirit …` is refused naming the id and
/// dispatches nothing.
#[test]
fn at_directive_is_refused_while_a_halt_is_pending() {
    let (host, handle) = FakeHost::ok(1);
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let repl = LiveRepl::start(host, handle, Arc::clone(&prompts));
    let id = raise_live(&repl);

    repl.input.send_line("@hello-spirit say hi");
    repl.wait_for("a halt is pending", "refusal");
    let text = repl.out.text();
    assert!(
        text.contains(&id),
        "the refusal names the pending id; output:\n{text}"
    );
    assert!(
        prompts.lock().is_empty(),
        "a refused @-line dispatches nothing"
    );
}

/// Trap 5's context race: state `Resumed`, context appearing a few ticks
/// LATER — the Spirit proceeds WITH it once it lands.
#[test]
fn context_arriving_late_is_waited_for_and_used() {
    let (host, handle) = FakeHost::ok(1);
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let repl = LiveRepl::start(host, handle, Arc::clone(&prompts));
    let id = raise_live(&repl);

    // Flip the state; the context lands only after the resolution render
    // would already have been waiting:
    repl.handle.set_state(&id, HaltState::Resumed);
    std::thread::sleep(std::time::Duration::from_millis(300));
    repl.handle.deliver_context(&id, "late context");
    repl.wait_for(
        &format!("halt {id} resolved: provided_context"),
        "resolution",
    );
    repl.wait_for("mock response", "proceeding turn");
    assert!(
        prompts.lock()[0].contains("late context"),
        "the late context is the one used"
    );
}

/// Trap 5's bound: state `Resumed`, context NEVER arriving — after the bound
/// the REPL says so and makes NO inference call. Never proceed empty.
#[test]
fn context_never_arriving_renders_not_delivered_and_never_proceeds() {
    let (host, handle) = FakeHost::ok(1);
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let repl = LiveRepl::start(host, handle, Arc::clone(&prompts));
    let id = raise_live(&repl);

    repl.handle.set_state(&id, HaltState::Resumed);
    repl.wait_for(
        &format!("halt {id} resolved: provided_context (context not delivered)"),
        "context not delivered",
    );
    let text = repl.out.text();
    assert!(
        !text.contains("mock response"),
        "no proceeding turn without context; output:\n{text}"
    );
    assert!(prompts.lock().is_empty(), "no inference call was made");
}
