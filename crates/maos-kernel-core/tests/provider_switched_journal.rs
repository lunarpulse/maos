use maos_domain::invariants::i10::{JournalEntry, LifecycleEvent};
use maos_domain::ports::scheduler::SpiritSchedulerPort;
use maos_kernel_core::security::manifest::{
    CapabilitiesRequired, ClassSection, LifecycleSection, OnCrashSection, OnRevocationSection,
    OutputShape, PostureSection, ProviderCapabilities, ProvidersSection, ResourceCaps,
    SandboxConfig, SchedulingSection, SupervisionSection,
};
use maos_kernel_core::security::SecurityManagerAdapter;

mod common;

struct MockJournal {
    entries: std::sync::Mutex<Vec<JournalEntry>>,
}

impl MockJournal {
    fn new() -> Self {
        Self {
            entries: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl SpiritSchedulerPort for MockJournal {
    fn journal_lifecycle(&self, entry: JournalEntry) {
        self.entries.lock().unwrap().push(entry);
    }

    fn last_lifecycle_event(&self, _spirit_id: &str) -> Option<LifecycleEvent> {
        None
    }
}

fn default_caps_required() -> CapabilitiesRequired {
    CapabilitiesRequired {
        provider: ProviderCapabilities {
            complete: vec!["anthropic.default".into()],
        },
        // Story 5.5c added the `mcp` field; default to an empty MCP capability
        // set so this 5.5b regression test stays unchanged in spirit.
        mcp: maos_kernel_core::security::manifest::McpCapabilities { servers: vec![] },
    }
}

fn default_resources() -> ResourceCaps {
    ResourceCaps::default()
}

fn default_posture() -> PostureSection {
    PostureSection::from_toml_str(
        r#"default = "cautious"
allowed_max = "cautious""#,
    )
    .unwrap()
}

fn admit_with_provider(
    security: &SecurityManagerAdapter,
    journal: &MockJournal,
    spirit_id: &str,
    providers: Option<&ProvidersSection>,
) {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let manifest = SandboxConfig::default();
    let caps = default_resources();
    let caps_req = default_caps_required();
    let posture = default_posture();

    security
        .admit_spirit(
            1,
            spirit_id,
            &manifest,
            &caps,
            &caps_req,
            None,
            journal,
            &posture,
            None,
            None,
            None,
            None,
            None,
            providers,
            Some(&ClassSection {
                name: "test-spirit".into(),
                version: "0.1.0".into(),
                abi: "1.0".into(),
                manifest_schema_version: maos_spirit_abi::MANIFEST_SCHEMA_VERSION,
                min_substrate_version: "0.0.1".into(),
                forms: vec!["rust-inproc".into()],
                trust_tier: "local".into(),
                description: "test".into(),
            }),
        )
        .unwrap();
}

#[test]
fn first_admit_emits_no_switch_event() {
    let security = SecurityManagerAdapter::default();
    let journal = MockJournal::new();

    let providers = ProvidersSection::from_toml_str(
        r#"
[primary]
id = "anthropic"
"#,
    )
    .unwrap();

    admit_with_provider(&security, &journal, "spirit-a", Some(&providers));

    let entries = journal.entries.lock().unwrap();
    let switch_events: Vec<_> = entries
        .iter()
        .filter(|e| {
            matches!(e, JournalEntry::Lifecycle(le) if le.lifecycle_event == LifecycleEvent::ProviderSwitched)
        })
        .collect();
    assert!(
        switch_events.is_empty(),
        "first admit should not emit ProviderSwitched"
    );
}

#[test]
fn second_admit_same_provider_emits_no_switch_event() {
    let security = SecurityManagerAdapter::default();
    let journal = MockJournal::new();

    let providers = ProvidersSection::from_toml_str(
        r#"
[primary]
id = "anthropic"
"#,
    )
    .unwrap();

    admit_with_provider(&security, &journal, "spirit-a", Some(&providers));
    admit_with_provider(&security, &journal, "spirit-a", Some(&providers));

    let entries = journal.entries.lock().unwrap();
    let switch_events: Vec<_> = entries
        .iter()
        .filter(|e| {
            matches!(e, JournalEntry::Lifecycle(le) if le.lifecycle_event == LifecycleEvent::ProviderSwitched)
        })
        .collect();
    assert!(
        switch_events.is_empty(),
        "same provider should not emit ProviderSwitched"
    );
}

#[test]
fn admit_with_changed_provider_emits_switch_event() {
    let security = SecurityManagerAdapter::default();
    let journal = MockJournal::new();

    let anthropic = ProvidersSection::from_toml_str(
        r#"
[primary]
id = "anthropic"
"#,
    )
    .unwrap();
    let openai = ProvidersSection::from_toml_str(
        r#"
[primary]
id = "openai"
"#,
    )
    .unwrap();

    admit_with_provider(&security, &journal, "spirit-a", Some(&anthropic));
    admit_with_provider(&security, &journal, "spirit-a", Some(&openai));

    let entries = journal.entries.lock().unwrap();
    let switch_events: Vec<_> = entries
        .iter()
        .filter_map(|e| match e {
            JournalEntry::Lifecycle(le)
                if le.lifecycle_event == LifecycleEvent::ProviderSwitched =>
            {
                Some(le.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        switch_events.len(),
        1,
        "should emit exactly one ProviderSwitched"
    );
    assert_eq!(switch_events[0].spirit_id, "spirit-a");
    assert!(
        switch_events[0].timestamp >= 1,
        "applied_at_ns should be >= 1 (monotonic_now_ns)"
    );
}

#[test]
fn multiple_switches_each_emit_event() {
    let security = SecurityManagerAdapter::default();
    let journal = MockJournal::new();

    let anthropic = ProvidersSection::from_toml_str(
        r#"
[primary]
id = "anthropic"
"#,
    )
    .unwrap();
    let openai = ProvidersSection::from_toml_str(
        r#"
[primary]
id = "openai"
"#,
    )
    .unwrap();
    let ollama = ProvidersSection::from_toml_str(
        r#"
[primary]
id = "ollama"
"#,
    )
    .unwrap();

    admit_with_provider(&security, &journal, "spirit-a", Some(&anthropic));
    admit_with_provider(&security, &journal, "spirit-a", Some(&openai));
    admit_with_provider(&security, &journal, "spirit-a", Some(&ollama));

    let entries = journal.entries.lock().unwrap();
    let switch_events: Vec<_> = entries
        .iter()
        .filter_map(|e| match e {
            JournalEntry::Lifecycle(le)
                if le.lifecycle_event == LifecycleEvent::ProviderSwitched =>
            {
                Some(le.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        switch_events.len(),
        2,
        "should emit two ProviderSwitched events"
    );
}
