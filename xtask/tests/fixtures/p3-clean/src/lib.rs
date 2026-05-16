// Fixture for Story 2.2 AC3 — pass scenario. NOT compiled by workspace.

#[maos_attrs::i9_exempt(reason = "Story 2.2 P3 clean fixture")]
pub struct P3CleanAdapter {
    inner: std::sync::Arc<dashmap::DashMap<String, u32>>,
}
