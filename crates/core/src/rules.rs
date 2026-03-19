use crate::error::{CoreError, CoreResult};
use crate::reimports::Message;
use smol_str::SmolStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Message type filter for conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}


impl MessageType {
    fn matches(&self, msg: &Message) -> bool {
        match (self, msg) {
            (MessageType::Text, Message::Text(_)) => true,
            (MessageType::Binary, Message::Binary(_)) => true,
            (MessageType::Close, Message::Close(_)) => true,
            (MessageType::Ping, Message::Ping(_)) => true,
            (MessageType::Pong, Message::Pong(_)) => true,
            _ => false,
        }
    }
}

impl TryFrom<String> for MessageType {
    type Error = CoreError;
    
    fn try_from(value: String) -> CoreResult<MessageType> {
        match value.as_str() {
            "Text" => Ok(MessageType::Text),
            "Binary" => Ok(MessageType::Binary),
            "Close" => Ok(MessageType::Close),
            "Ping" => Ok(MessageType::Ping),
            "Pong" => Ok(MessageType::Pong),
            _ => Err(CoreError::Other(format!("Invalid message type: {}", value))),
        }
    }
}

/// Text matching strategies.
#[derive(Debug, Clone)]
pub enum TextMatcher {
    StartsWith(SmolStr),
    EndsWith(SmolStr),
    Contains(SmolStr),
    Exact(SmolStr),
    Regex(String),
}

impl TextMatcher {
    fn call(&self, text: &str) -> bool {
        match self {
            TextMatcher::StartsWith(s) => text.starts_with(s.as_str()),
            TextMatcher::EndsWith(s) => text.ends_with(s.as_str()),
            TextMatcher::Contains(s) => text.contains(s.as_str()),
            TextMatcher::Exact(s) => text == s.as_str(),
            TextMatcher::Regex(pattern) => regex::Regex::new(pattern)
                .map(|re| re.is_match(text))
                .unwrap_or(false),
        }
    }
}

/// Binary matching strategies.
#[derive(Debug, Clone)]
pub enum BinaryMatcher {
    StartsWith(Vec<u8>),
    EndsWith(Vec<u8>),
    Contains(Vec<u8>),
    Exact(Vec<u8>),
}

impl BinaryMatcher {
    fn call(&self, data: &[u8]) -> bool {
        match self {
            BinaryMatcher::StartsWith(prefix) => data.starts_with(prefix),
            BinaryMatcher::EndsWith(suffix) => data.ends_with(suffix),
            BinaryMatcher::Contains(needle) => data.windows(needle.len()).any(|w| w == needle),
            BinaryMatcher::Exact(expected) => data == expected,
        }
    }
}

/// Core matchers for different message parts.
#[derive(Clone)]
pub enum Matcher {
    Text(TextMatcher),
    Binary(BinaryMatcher),
    MessageType(MessageType),
    JsonSchema(Arc<dyn Fn(&serde_json::Value) -> bool + Send + Sync>),
    Any,
    Never,
    Custom(Arc<dyn Fn(&Message) -> bool + Send + Sync>),
}

impl Matcher {
    fn call(&self, msg: &Message) -> bool {
        match self {
            Matcher::Text(tm) => match msg {
                Message::Text(text) => tm.call(text.as_str()),
                Message::Binary(data) => String::from_utf8(data.to_vec())
                    .ok()
                    .as_ref()
                    .map(|s| tm.call(s))
                    .unwrap_or(false),
                _ => false,
            },
            Matcher::Binary(bm) => match msg {
                Message::Binary(data) => bm.call(data),
                _ => false,
            },
            Matcher::MessageType(mt) => mt.matches(msg),
            Matcher::JsonSchema(f) => {
                let text = match msg {
                    Message::Text(text) => Some(text.as_str()),
                    Message::Binary(data) => std::str::from_utf8(data).ok(),
                    _ => None,
                };

                if let Some(t) = text {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(t) {
                        return f(&val);
                    }
                }
                false
            }
            Matcher::Any => true,
            Matcher::Never => false,
            Matcher::Custom(f) => f(msg),
        }
    }
}

/// Conditions that gate when a matcher result is acted upon.
#[derive(Clone)]
pub enum Condition {
    Always,
    Wait(Arc<AtomicUsize>),
    WaitMessages {
        period: usize,
        offset: usize,
        counter: Arc<AtomicUsize>,
    },
    Custom(Arc<dyn Fn(&Message) -> bool + Send + Sync>),
}

impl Condition {
    fn call(&self, msg: &Message) -> bool {
        match self {
            Condition::Always => true,
            Condition::Wait(wait) => {
                let current = wait.load(Ordering::SeqCst);
                if current > 0 {
                    wait.store(current - 1, Ordering::SeqCst);
                    false
                } else {
                    true
                }
            }
            Condition::WaitMessages {
                period,
                offset,
                counter,
            } => {
                let current = counter.fetch_add(1, Ordering::SeqCst);
                current % period == *offset
            }
            Condition::Custom(f) => f(msg),
        }
    }

    fn reset(&self) {
        match self {
            Condition::Wait(wait) => wait.store(0, Ordering::SeqCst),
            Condition::WaitMessages { counter, .. } => counter.store(0, Ordering::SeqCst),
            _ => {}
        }
    }
}

/// Modifiers that transform the message before passing to inner rules.
#[derive(Clone)]
pub enum Modifier {
    LstripThen { prefix: SmolStr, inner: Box<Rule> },
    RstripThen { suffix: SmolStr, inner: Box<Rule> },
    LstripUntil { target: SmolStr, inner: Box<Rule> },
    RstripUntil { target: SmolStr, inner: Box<Rule> },
    Custom(Arc<dyn Fn(&Message) -> Option<Message> + Send + Sync>),
}

impl Modifier {
    fn call(&self, msg: &Message) -> Option<Message> {
        match self {
            Modifier::LstripThen { prefix, inner } => {
                let text = Self::get_text_owned(msg)?;
                if let Some(stripped) = text.strip_prefix(prefix.as_str()) {
                    let new_msg = Message::Text(stripped.into());
                    if inner.call(&new_msg) {
                        return Some(new_msg);
                    }
                }
                None
            }
            Modifier::RstripThen { suffix, inner } => {
                let text = Self::get_text_owned(msg)?;
                if let Some(stripped) = text.strip_suffix(suffix.as_str()) {
                    let new_msg = Message::Text(stripped.into());
                    if inner.call(&new_msg) {
                        return Some(new_msg);
                    }
                }
                None
            }
            Modifier::LstripUntil { target, inner } => {
                let text = Self::get_text_owned(msg)?;
                if let Some(start) = text.find(target.as_str()) {
                    let new_msg = Message::Text(text[start..].into());
                    if inner.call(&new_msg) {
                        return Some(new_msg);
                    }
                }
                None
            }
            Modifier::RstripUntil { target, inner } => {
                let text = Self::get_text_owned(msg)?;
                if let Some(end) = text.rfind(target.as_str()) {
                    let new_msg = Message::Text(text[..=end + target.len() - 1].into());
                    if inner.call(&new_msg) {
                        return Some(new_msg);
                    }
                }
                None
            }
            Modifier::Custom(f) => f(msg),
        }
    }

    fn get_text_owned(msg: &Message) -> Option<String> {
        match msg {
            Message::Text(t) => Some(t.to_string()),
            Message::Binary(b) => String::from_utf8(b.to_vec()).ok(),
            _ => None,
        }
    }

    fn reset(&self) {
        match self {
            Modifier::LstripThen { inner, .. } => inner.reset(),
            Modifier::RstripThen { inner, .. } => inner.reset(),
            Modifier::LstripUntil { inner, .. } => inner.reset(),
            Modifier::RstripUntil { inner, .. } => inner.reset(),
            Modifier::Custom(_) => {}
        }
    }
}

/// Logical combinators for composing rules.
#[derive(Clone)]
pub enum Combinator {
    Or(Vec<Rule>),
    And(Vec<Rule>),
    Then {
        first: Box<Rule>,
        second: Box<Rule>,
        waiting: Arc<AtomicUsize>,
    },
    AndThen {
        first: Box<Rule>,
        second: Box<Rule>,
    },
    Not(Box<Rule>),
}

impl Combinator {
    fn call(&self, msg: &Message) -> bool {
        match self {
            Combinator::Or(rules) => rules.iter().any(|r| r.call(msg)),
            Combinator::And(rules) => rules.iter().all(|r| r.call(msg)),
            Combinator::Then {
                first,
                second,
                waiting,
            } => {
                let is_waiting = waiting.load(Ordering::SeqCst) > 0;
                if is_waiting {
                    waiting.store(0, Ordering::SeqCst);
                    second.call(msg)
                } else if first.call(msg) {
                    waiting.store(1, Ordering::SeqCst);
                    false
                } else {
                    false
                }
            }
            Combinator::AndThen { first, second } => first.call(msg) && second.call(msg),
            Combinator::Not(rule) => !rule.call(msg),
        }
    }

    fn reset(&self) {
        match self {
            Combinator::Or(rules) => rules.iter().for_each(|r| r.reset()),
            Combinator::And(rules) => rules.iter().for_each(|r| r.reset()),
            Combinator::Then {
                first,
                second,
                waiting,
            } => {
                waiting.store(0, Ordering::SeqCst);
                first.reset();
                second.reset();
            }
            Combinator::AndThen { first, second } => {
                first.reset();
                second.reset();
            }
            Combinator::Not(rule) => rule.reset(),
        }
    }
}

/// Main rule type that implements the trait.
#[derive(Clone)]
pub enum Rule {
    Matcher {
        matcher: Matcher,
        conditions: Vec<Condition>,
    },
    ModifiedRule(Modifier),
    Combinator(Combinator),
    Custom(Arc<dyn Fn(&Message) -> bool + Send + Sync>),
}

impl Rule {
    pub fn call(&self, msg: &Message) -> bool {
        match self {
            Rule::Matcher {
                matcher,
                conditions,
            } => {
                if !matcher.call(msg) {
                    return false;
                }
                for cond in conditions {
                    if !cond.call(msg) {
                        return false;
                    }
                }
                true
            }
            Rule::ModifiedRule(modifier) => modifier.call(msg).is_some(),
            Rule::Combinator(comb) => comb.call(msg),
            Rule::Custom(f) => f(msg),
        }
    }

    pub fn reset(&self) {
        match self {
            Rule::Matcher { conditions, .. } => {
                for cond in conditions {
                    cond.reset();
                }
            }
            Rule::ModifiedRule(modifier) => modifier.reset(),
            Rule::Combinator(comb) => comb.reset(),
            Rule::Custom(_) => {}
        }
    }
}

impl crate::traits::Rule for Rule {
    fn call(&self, msg: &Message) -> bool {
        Rule::call(self, msg)
    }

    fn reset(&self) {
        Rule::reset(self)
    }
}

/// Builder for creating rules with a fluent API.
pub struct RuleBuilder {
    inner: Rule,
}

impl RuleBuilder {
    // === Matcher constructors ===

    pub fn text_starts_with(s: impl Into<SmolStr>) -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::Text(TextMatcher::StartsWith(s.into())),
                conditions: vec![],
            },
        }
    }

    pub fn text_ends_with(s: impl Into<SmolStr>) -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::Text(TextMatcher::EndsWith(s.into())),
                conditions: vec![],
            },
        }
    }

    pub fn text_contains(s: impl Into<SmolStr>) -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::Text(TextMatcher::Contains(s.into())),
                conditions: vec![],
            },
        }
    }

    pub fn text_exact(s: impl Into<SmolStr>) -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::Text(TextMatcher::Exact(s.into())),
                conditions: vec![],
            },
        }
    }

    pub fn text_regex(pattern: impl Into<String>) -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::Text(TextMatcher::Regex(pattern.into())),
                conditions: vec![],
            },
        }
    }

    pub fn binary_starts_with(data: impl Into<Vec<u8>>) -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::Binary(BinaryMatcher::StartsWith(data.into())),
                conditions: vec![],
            },
        }
    }

    pub fn binary_ends_with(data: impl Into<Vec<u8>>) -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::Binary(BinaryMatcher::EndsWith(data.into())),
                conditions: vec![],
            },
        }
    }

    pub fn binary_contains(data: impl Into<Vec<u8>>) -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::Binary(BinaryMatcher::Contains(data.into())),
                conditions: vec![],
            },
        }
    }

    pub fn binary_exact(data: impl Into<Vec<u8>>) -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::Binary(BinaryMatcher::Exact(data.into())),
                conditions: vec![],
            },
        }
    }

    pub fn message_type(mt: MessageType) -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::MessageType(mt),
                conditions: vec![],
            },
        }
    }

    pub fn any() -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::Any,
                conditions: vec![],
            },
        }
    }

    pub fn never() -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::Never,
                conditions: vec![],
            },
        }
    }

    pub fn custom(f: impl Fn(&Message) -> bool + Send + Sync + 'static) -> Self {
        Self {
            inner: Rule::Custom(Arc::new(f)),
        }
    }

    pub fn json_schema<T: serde::de::DeserializeOwned + 'static>() -> Self {
        Self {
            inner: Rule::Matcher {
                matcher: Matcher::JsonSchema(Arc::new(|val| {
                    serde_json::from_value::<T>(val.clone()).is_ok()
                })),
                conditions: vec![],
            },
        }
    }

    // === Condition modifiers ===

    pub fn wait(self, n: usize) -> Self {
        self.with_condition(Condition::Wait(Arc::new(AtomicUsize::new(n))))
    }

    pub fn wait_messages(self, period: usize) -> Self {
        self.wait_messages_with_offset(period, 0)
    }

    pub fn wait_messages_with_offset(self, period: usize, offset: usize) -> Self {
        self.with_condition(Condition::WaitMessages {
            period,
            offset,
            counter: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn with_condition(mut self, cond: Condition) -> Self {
        match &mut self.inner {
            Rule::Matcher { conditions, .. } => {
                conditions.push(cond);
            }
            _ => {
                // Can only apply conditions to matchers
                return self;
            }
        }
        self
    }

    // === Modifiers ===

    pub fn lstrip_then(self, prefix: impl Into<SmolStr>) -> Self {
        let prefix_str = prefix.into();
        Self {
            inner: Rule::ModifiedRule(Modifier::LstripThen {
                prefix: prefix_str,
                inner: Box::new(self.inner),
            }),
        }
    }

    pub fn rstrip_then(self, suffix: impl Into<SmolStr>) -> Self {
        let suffix_str = suffix.into();
        Self {
            inner: Rule::ModifiedRule(Modifier::RstripThen {
                suffix: suffix_str,
                inner: Box::new(self.inner),
            }),
        }
    }

    pub fn lstrip_until(self, target: impl Into<SmolStr>) -> Self {
        let target_str = target.into();
        Self {
            inner: Rule::ModifiedRule(Modifier::LstripUntil {
                target: target_str,
                inner: Box::new(self.inner),
            }),
        }
    }

    pub fn rstrip_until(self, target: impl Into<SmolStr>) -> Self {
        let target_str = target.into();
        Self {
            inner: Rule::ModifiedRule(Modifier::RstripUntil {
                target: target_str,
                inner: Box::new(self.inner),
            }),
        }
    }

    // === Combinators ===

    pub fn or(self, other: Rule) -> Self {
        Self {
            inner: Rule::Combinator(Combinator::Or(vec![self.inner, other])),
        }
    }

    pub fn or_many(rules: Vec<Rule>) -> Self {
        Self {
            inner: Rule::Combinator(Combinator::Or(rules)),
        }
    }

    pub fn and(self, other: Rule) -> Self {
        Self {
            inner: Rule::Combinator(Combinator::And(vec![self.inner, other])),
        }
    }

    pub fn and_many(rules: Vec<Rule>) -> Self {
        Self {
            inner: Rule::Combinator(Combinator::And(rules)),
        }
    }

    pub fn then(self, next: Rule) -> Self {
        Self {
            inner: Rule::Combinator(Combinator::Then {
                first: Box::new(self.inner),
                second: Box::new(next),
                waiting: Arc::new(AtomicUsize::new(0)),
            }),
        }
    }

    pub fn and_then(self, next: Rule) -> Self {
        Self {
            inner: Rule::Combinator(Combinator::AndThen {
                first: Box::new(self.inner),
                second: Box::new(next),
            }),
        }
    }

    pub fn not(self) -> Self {
        Self {
            inner: Rule::Combinator(Combinator::Not(Box::new(self.inner))),
        }
    }

    pub fn build(self) -> Rule {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_starts_with() {
        let rule = RuleBuilder::text_starts_with("451").build();
        let msg = Message::Text("451-data".into());
        assert!(rule.call(&msg));

        let msg2 = Message::Text("123-data".into());
        assert!(!rule.call(&msg2));
    }

    #[test]
    fn test_wait_condition() {
        let rule = RuleBuilder::any().wait(2).build();
        let msg = Message::Text("test".into());

        assert!(!rule.call(&msg)); // wait=2, decrement to 1, return false
        assert!(!rule.call(&msg)); // wait=1, decrement to 0, return false
        assert!(rule.call(&msg)); // wait=0, return true
        assert!(rule.call(&msg)); // wait=0, return true
    }

    #[test]
    fn test_wait_messages() {
        let rule = RuleBuilder::any().wait_messages(2).build();
        let msg = Message::Text("test".into());

        assert!(rule.call(&msg)); // counter=0, 0 % 2 == 0, true
        assert!(!rule.call(&msg)); // counter=1, 1 % 2 != 0, false
        assert!(rule.call(&msg)); // counter=2, 2 % 2 == 0, true
        assert!(!rule.call(&msg)); // counter=3, 3 % 2 != 0, false
    }

    #[test]
    fn test_wait_messages_with_offset() {
        let rule = RuleBuilder::any().wait_messages_with_offset(2, 1).build();
        let msg = Message::Text("test".into());

        assert!(!rule.call(&msg)); // counter=0, 0 % 2 == 0 != 1, false
        assert!(rule.call(&msg)); // counter=1, 1 % 2 == 1, true
        assert!(!rule.call(&msg)); // counter=2, 2 % 2 == 0 != 1, false
        assert!(rule.call(&msg)); // counter=3, 3 % 2 == 1, true
    }

    #[test]
    fn test_then_combinator() {
        let rule = RuleBuilder::text_starts_with("451")
            .then(RuleBuilder::any().build())
            .build();
        let msg1 = Message::Text("451-data".into());
        let msg2 = Message::Text("other".into());

        assert!(!rule.call(&msg1)); // first matches, set waiting, return false
        assert!(rule.call(&msg2)); // waiting, any matches, return true
        assert!(!rule.call(&msg2)); // not waiting, return false
    }

    #[test]
    fn test_or_combinator() {
        let rule = RuleBuilder::text_starts_with("451")
            .or(RuleBuilder::text_starts_with("123").build())
            .build();
        let msg1 = Message::Text("451-data".into());
        let msg2 = Message::Text("123-data".into());
        let msg3 = Message::Text("999-data".into());

        assert!(rule.call(&msg1));
        assert!(rule.call(&msg2));
        assert!(!rule.call(&msg3));
    }

    #[test]
    fn test_and_combinator() {
        let rule = RuleBuilder::text_starts_with("451")
            .and(RuleBuilder::text_contains("data").build())
            .build();
        let msg1 = Message::Text("451-data".into());
        let msg2 = Message::Text("451-other".into());

        assert!(rule.call(&msg1));
        assert!(!rule.call(&msg2));
    }

    #[test]
    fn test_lstrip_then() {
        let rule = RuleBuilder::text_contains("data")
            .lstrip_then("451-")
            .build();
        let msg = Message::Text("451-data".into());
        assert!(rule.call(&msg));

        let msg2 = Message::Text("451-other".into());
        assert!(!rule.call(&msg2));
    }

    #[derive(serde::Deserialize)]
    struct TestData {
        #[allow(dead_code)]
        id: u32,
    }

    #[test]
    fn test_json_schema() {
        let rule = RuleBuilder::json_schema::<TestData>().build();

        let msg_valid = Message::Text(r#"{"id": 42}"#.to_string().into());
        let msg_invalid_json = Message::Text(r#"{"id": }"#.to_string().into());
        let msg_wrong_schema = Message::Text(r#"{"name": "test"}"#.to_string().into());
        let msg_bin = Message::Binary(br#"{"id": 123}"#.to_vec().into());

        assert!(rule.call(&msg_valid));
        assert!(!rule.call(&msg_invalid_json));
        assert!(!rule.call(&msg_wrong_schema));
        assert!(rule.call(&msg_bin));
    }

    #[test]
    fn test_not_combinator() {
        let rule = RuleBuilder::text_starts_with("451").not().build();
        let msg1 = Message::Text("451-data".into());
        let msg2 = Message::Text("123-data".into());

        assert!(!rule.call(&msg1));
        assert!(rule.call(&msg2));
    }
}
