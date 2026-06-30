//! Procedural macros for `binary_options_tools_core`.
//!
//! This crate exposes the `Rule` attribute macro used to generate concrete
//! `Rule` implementations from either:
//!
//! - an existing rule type (`#[rule(MyRuleType)]`),
//! - a factory function (`#[rule(fn = make_rule)]`),
//! - or the rule DSL (`#[rule({ ...dsl... })]`).
//!
//! The generated type always wraps an inner `Box<dyn Rule + Send + Sync>` and
//! delegates `call` and `reset` to it.

use crate::rule::RuleExpr;

mod modules;
mod rule;

/// Generates a concrete rule wrapper type from a `struct` declaration.
///
/// # Usage
///
/// You apply two attributes to a unit-like struct:
///
/// - `#[Rule]` (this macro)
/// - `#[rule(...)]` (payload consumed by this macro)
///
/// ## Example: struct rule constructor
///
/// ```ignore
/// #[Rule]
/// #[rule(MyExistingRule)]
/// pub struct MyRule;
/// ```
///
/// Expands to a wrapper whose `new()` constructs:
///
/// ```ignore
/// Box::new(MyExistingRule::new())
/// ```
///
/// ## Example: function factory
///
/// ```ignore
/// #[Rule]
/// #[rule(fn = make_my_rule)]
/// pub struct MyRule;
/// ```
///
/// Expands to:
///
/// ```ignore
/// Box::new(make_my_rule())
/// ```
///
/// ## Example: DSL mode
///
/// ```ignore
/// #[Rule]
/// #[rule({ starts_with("42") & !contains("error") })]
/// pub struct TradeRule;
/// ```
///
/// # DSL Quick Reference
///
/// The DSL supports:
///
/// - Matchers:
///   - `any()`
///   - `never()`
///   - `exact(...)`
///   - `starts_with(...)`
///   - `ends_with(...)`
///   - `contains(...)`
///   - `regex(...)`
///   - `binary_exact(...)`
///   - `binary_starts_with(...)`
///   - `binary_ends_with(...)`
///   - `binary_contains(...)`
///   - `message_type(...)`
///   - `json_schema(...)`
///   - `custom(|msg| { ... })`
///
/// - Combinators/operators:
///   - `a & b` (AND)
///   - `a | b` (OR)
///   - `a -> b` (THEN/sequence)
///   - `!a` (NOT)
///   - `( ... )` grouping
///
/// - Chained builder methods after a matcher:
///   - `starts_with("x").wait(1)`
///   - `contains("{").lstrip_until("{")`
///
/// # Operator precedence
///
/// Highest to lowest:
///
/// 1. Parentheses and matcher/method chains
/// 2. `!`
/// 3. `&`
/// 4. `|`
/// 5. `->`
///
/// # Constant/identifier matcher args
///
/// Text-based matcher args (`exact`, `starts_with`, `ends_with`, `contains`,
/// `regex`) accept either:
///
/// - string literals: `starts_with("42")`
/// - identifiers/constants: `starts_with(PREFIX_42)`
///
/// This allows sharing reusable constants across rules.
///
/// # Generated code shape
///
/// Given:
///
/// ```ignore
/// #[Rule]
/// #[rule({ starts_with("A") | ends_with("Z") })]
/// pub struct AlphaRule;
/// ```
///
/// The macro generates roughly:
///
/// ```ignore
/// pub struct AlphaRule {
///     inner: Box<dyn ::binary_options_tools_core::traits::Rule + Send + Sync>
/// }
///
/// impl AlphaRule {
///     pub fn new() -> Self {
///         Self { inner: /* compiled DSL */ }
///     }
/// }
///
/// impl ::binary_options_tools_core::traits::Rule for AlphaRule {
///     fn call(&self, msg: &::binary_options_tools_core::reimports::Message) -> bool {
///         self.inner.call(msg)
///     }
///
///     fn reset(&self) {
///         self.inner.reset()
///     }
/// }
/// ```
///
/// # Errors
///
/// If `#[rule(...)]` is missing or malformed, macro expansion returns a compile
/// error with parser context from the DSL parser.
#[zyn::attribute]
pub fn rule(
    #[zyn(input)] vis: zyn::Extract<zyn::syn::Visibility>,
    #[zyn(input)] ident: zyn::syn::ItemStruct,
    #[zyn(input)] attr: RuleExpr,
) -> zyn::TokenStream {
    let ident = ident.ident;
    let vis = vis.inner();

    zyn::zyn! {
        {{ vis }} struct {{ ident }} {
            inner: Box<dyn ::binary_options_tools_core::traits::Rule + Send + Sync>
        }

        impl {{ ident }} {
            {{ vis }} fn new() -> Self {
                Self {
                    inner: {{ attr.to_tokens() }},
                }
            }
        }

        impl ::binary_options_tools_core::traits::Rule for {{ ident }} {
            fn call(&self, msg: &::binary_options_tools_core::reimports::Message) -> bool {
                self.inner.call(msg)
            }

            fn reset(&self) {
                self.inner.reset()
            }
        }
    }
}
