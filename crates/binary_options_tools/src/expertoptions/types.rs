use binary_options_tools_core_pre::traits::Rule;

pub struct MultiRule {
    rules: Vec<Box<dyn Rule + Send + Sync>>,
}

impl MultiRule {
    pub fn new(rules: Vec<Box<dyn Rule + Send + Sync>>) -> Self {
        Self { rules }
    }
}

impl Rule for MultiRule {
    fn call(&self, msg: &binary_options_tools_core_pre::reimports::Message) -> bool {
        for rule in &self.rules {
            if rule.call(msg) {
                return true;
            }
        }
        false
    }

    fn reset(&self) {
        for rule in &self.rules {
            rule.reset();
        }
    }
}