pub trait ValidatorTrait {
    /// Validates the given data and returns a boolean indicating if the data is valid or not.
    fn call(&self, data: &str) -> bool;
}

impl<T: Fn(&str) -> bool + Send + Sync + 'static> ValidatorTrait for T {
    fn call(&self, data: &str) -> bool {
        self(data)
    }
}
