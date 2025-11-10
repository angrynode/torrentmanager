pub struct Category(String);

impl Category {
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }
}
