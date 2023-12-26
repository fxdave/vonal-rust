#[derive(Default)]
pub struct Cached<T: Clone> {
    pub key: String,
    pub value: Option<T>,
}

impl<T: Clone> Cached<T> {
    pub fn get_or_create(&mut self, key: String, get_new_value: impl FnOnce() -> T) -> T {
        if self.value.is_none() || self.key != key {
            self.key = key;
            self.value = Some(get_new_value());
        }
        self.value.as_ref().unwrap().clone()
    }
}
