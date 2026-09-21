// Story 15-1 AC2 residual control (§10): the exemption-documentation leg had
// NO test through the gate — `missing_exemption_doc_fires` reimplements the
// matcher inline and has already diverged, and `clean_i9_passes`'s fixture has
// no exempt struct at all. This fixture is an `#[i9_exempt]` site whose name
// appears NOWHERE in docs/invariants/i9-exemptions.md, so the gate must red on
// the undocumented-exemption leg specifically.
#[i9_exempt(reason = "fixture; deliberately undocumented residual control")]
pub struct UndocumentedExemptFixture {
    inner: std::sync::Mutex<Vec<String>>,
}
