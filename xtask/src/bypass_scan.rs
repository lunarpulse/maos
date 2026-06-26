//! Story 9.4b AC-9 / R-RG3 — region write/read bypass scanner.
//!
//! Option A makes the working-memory rows plaintext and region-bound by
//! *governance* (the `WriteEntryPoint` chokepoint + region-verified audit
//! frames), with NO cryptographic at-rest backstop.  So the chokepoint is the
//! SOLE safety case for the memory layer (Murat).  This gate is the runtime
//! companion to the compile-time non-wildcard enum: it reds if
//!
//! 1. a raw store write/read method becomes bare `pub` (externally reachable,
//!    able to bypass the chokepoint — this is also the structural half of the
//!    no-unanchored-read proof, R1-COND), or
//! 2. `write_entry_point::enforce_region` grows a wildcard `_ =>` match arm, or
//! 3. the adapter write path stops routing through `enforce_region`.

use regex::Regex;
use std::fs;
use std::path::Path;

/// Raw store methods that MUST be `pub(in crate::memory)` (or tighter) so the
/// only public read/write surface is the region-aware `MemoryManagerAdapter`.
const GUARDED_METHODS: &[(&str, &[&str])] = &[
    ("private.rs", &["fn write(", "fn read(", "fn scan("]),
    ("shared.rs", &["fn write(", "fn read(", "fn scan("]),
    ("principal.rs", &["fn record_write(", "fn lookup("]),
];

pub fn run(json: bool) -> Result<(), String> {
    let mem = Path::new("crates/maos-kernel-core/src/memory");
    let mut violations: Vec<String> = Vec::new();

    // Regex: word-bounded `pub` keyword (matches both bare `pub` and `pub(...)`).
    let re_pub = Regex::new(r"\bpub\b").expect("bypass-scan: bad regex");
    // Regex: `pub(...)` — any scoped visibility (pub(crate), pub(super), pub(in ...)).
    let re_scoped_pub = Regex::new(r"\bpub\s*\(").expect("bypass-scan: bad regex");
    // Regex: wildcard match arm `_ =>` (with optional whitespace).
    let re_wildcard_arm = Regex::new(r"\b_\s*=>").expect("bypass-scan: bad regex");
    // Regex: `fn enforce_region` with word boundaries.
    let re_enforce_region_def =
        Regex::new(r"\bfn\s+enforce_region\b").expect("bypass-scan: bad regex");
    // Regex: call to `enforce_region(`.
    let re_enforce_region_call =
        Regex::new(r"\benforce_region\s*\(").expect("bypass-scan: bad regex");

    // (1) Raw store writers/readers must never be bare `pub`.
    for (file, methods) in GUARDED_METHODS {
        let path = mem.join(file);
        let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for method in *methods {
            // Build a regex for this specific method signature, e.g. `\bfn\s+write\s*\(`
            let fn_name = method.trim_start_matches("fn ").trim_end_matches('(');
            let re_method = Regex::new(&format!(r"\bfn\s+{}\s*\(", regex::escape(fn_name)))
                .expect("bypass-scan: bad method regex");
            let mut found = false;
            for (i, line) in src.lines().enumerate() {
                // Skip comment lines entirely — doc comments may mention method names.
                let trimmed = line.trim_start();
                if trimmed.starts_with("//")
                    || trimmed.starts_with('*')
                    || trimmed.starts_with("/*")
                {
                    continue;
                }
                if re_method.is_match(line) {
                    found = true;
                    // The line declares the method.  Check its visibility.
                    if re_pub.is_match(line) && !re_scoped_pub.is_match(line) {
                        violations.push(format!(
                            "{}:{}: `{}` is bare `pub` — raw store method must be \
                             pub(in crate::memory) so it cannot bypass the WriteEntryPoint \
                             chokepoint (AC-9 / no-unanchored-read)",
                            file,
                            i + 1,
                            line.trim()
                        ));
                    }
                }
            }
            if !found {
                violations.push(format!(
                    "{file}: expected guarded method `{method}` not found — \
                     scanner is stale, refusing to pass (AC-9)"
                ));
            }
        }
    }

    // (2) The enforcement match must stay non-wildcard.
    let wep_path = mem.join("write_entry_point.rs");
    let wep = fs::read_to_string(&wep_path).map_err(|e| format!("{}: {e}", wep_path.display()))?;
    // Ignore comment lines — doc comments legitimately mention `_ =>` in prose.
    let has_wildcard_arm = wep.lines().any(|line| {
        let t = line.trim_start();
        let is_comment = t.starts_with("//") || t.starts_with('*') || t.starts_with("/*");
        !is_comment && re_wildcard_arm.is_match(line)
    });
    if has_wildcard_arm {
        violations.push(
            "write_entry_point.rs: a wildcard `_ =>` arm is present — AC-9 requires an \
             exhaustive non-wildcard match so a new WriteEntryPoint variant fails to compile \
             until its region provenance is handled (R-RG3)"
                .to_string(),
        );
    }
    if !re_enforce_region_def.is_match(&wep) {
        violations.push("write_entry_point.rs: `enforce_region` not found".to_string());
    }

    // (3) The adapter write path must route through the chokepoint.
    let modrs_path = mem.join("mod.rs");
    let modrs =
        fs::read_to_string(&modrs_path).map_err(|e| format!("{}: {e}", modrs_path.display()))?;
    if !re_enforce_region_call.is_match(&modrs) {
        violations.push(
            "memory/mod.rs: the store write path does not call \
             write_entry_point::enforce_region — region chokepoint bypassed (AC-9)"
                .to_string(),
        );
    }

    let passed = violations.is_empty();
    if json {
        println!(
            "{}",
            serde_json::json!({
                "check": "bypass-scan",
                "passed": passed,
                "violations": violations,
            })
        );
    } else if passed {
        eprintln!(
            "bypass-scan: PASS — memory store raw methods are chokepoint-guarded, \
             enforce_region is wildcard-free, and the adapter routes through it"
        );
    } else {
        eprintln!("bypass-scan: FAIL");
        for v in &violations {
            eprintln!("  [!] {v}");
        }
    }

    if passed {
        Ok(())
    } else {
        Err(format!(
            "bypass-scan failed: {} violation(s)",
            violations.len()
        ))
    }
}
