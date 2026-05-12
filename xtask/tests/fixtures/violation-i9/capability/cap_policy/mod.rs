pub struct HungryCache {
    inner: std::collections::HashMap<String, Vec<Decision>>,
}

pub struct Decision {
    pub approved: bool,
}
