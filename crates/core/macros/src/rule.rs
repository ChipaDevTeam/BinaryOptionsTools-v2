use zyn::{
    FromInput, Input,
    syn::{
        self, Expr, ExprClosure, ExprPath, Ident, LitStr, Token, Type, braced, parenthesized,
        parse::Parse, token::Paren,
    },
};

/// Parsed form of the `#[rule(...)]` attribute payload.
///
/// The rule macro supports three input modes:
///
/// 1. Struct constructor mode:
///    - `#[rule(MyRuleType)]`
///    - Expands to `Box::new(MyRuleType::new())`.
///
/// 2. Function factory mode:
///    - `#[rule(fn = make_rule)]`
///    - Expands to `Box::new(make_rule())`.
///
/// 3. DSL mode:
///    - `#[rule({ starts_with("42") & !contains("error") })]`
///    - Parses and compiles a micro-language into `RuleBuilder` calls.
///
/// DSL mode is the intended “author-facing” mode for ergonomic rule definitions.
pub(crate) enum RuleExpr {
    /// A direct reference to a struct type: `rule(MyCustomRule)`
    Struct(Ident),

    /// A function path that returns a rule: `rule(fn = my_rule_maker)`
    FnRef(ExprPath),

    /// The DSL payload wrapped in braces:
    /// `rule({ starts_with("...") & contains("...") })`
    Dsl(DslNode),
}

/// AST node for the rule DSL expression tree.
///
/// ## Operator summary
///
/// - `a -> b` : sequence/then
/// - `a | b`  : OR
/// - `a & b`  : AND
/// - `!a`     : NOT
/// - `( ... )`: grouping
///
/// ## Precedence (highest → lowest)
///
/// 1. Parentheses / method chains
/// 2. `!`
/// 3. `&`
/// 4. `|`
/// 5. `->`
///
/// This is implemented by recursive descent:
/// `parse_then -> parse_or -> parse_and -> parse_not -> parse_base`.
pub(crate) enum DslNode {
    /// `a -> b`
    Then(Box<DslNode>, Box<DslNode>),

    /// `a | b | c` (flattened)
    Or(Vec<DslNode>),

    /// `a & b & c` (flattened)
    And(Vec<DslNode>),

    /// `!a`
    Not(Box<DslNode>),

    /// Terminal matcher expression with optional chained methods:
    /// `starts_with("42").wait(1).lstrip_until("{")`
    MethodChain(MethodChain),
}

/// Terminal DSL element:
/// - base matcher call (`starts_with("x")`, `any()`, `binary_contains([..])`, ...)
/// - followed by zero or more chained methods (`.wait(1)`, `.rstrip_then("...")`, ...)
pub(crate) struct MethodChain {
    pub base: MatcherCall,
    pub methods: Vec<ChainedMethod>,
}

/// Supported base matcher calls in DSL mode.
///
/// Text matchers now accept either string literals or identifiers/constants
/// (via `IdentOrString`) so both are valid:
/// - `starts_with("42")`
/// - `starts_with(PREFIX_42)`
pub(crate) enum MatcherCall {
    Any,                       // any()
    Never,                     // never()
    Exact(IdentOrString),      // exact("string") | exact(CONST)
    StartsWith(IdentOrString), // starts_with("string") | starts_with(CONST)
    EndsWith(IdentOrString),   // ends_with("string") | ends_with(CONST)
    Contains(IdentOrString),   // contains("string") | contains(CONST)
    Regex(IdentOrString),      // regex("pattern") | regex(PATTERN_CONST)

    BinaryExact(Expr),      // binary_exact([0x01, 0x02])
    BinaryStartsWith(Expr), // binary_starts_with([0x01])
    BinaryEndsWith(Expr),   // binary_ends_with([0x01])
    BinaryContains(Expr),   // binary_contains([0x01])

    MessageType(Ident), // message_type(Text)
    JsonSchema(Type),   // json_schema(MyStruct)

    Custom(ExprClosure), // custom(|msg| { ... })
}

/// Parsed chained method call in DSL mode.
/// Examples:
/// - `.wait(1)`
/// - `.lstrip_until("{")`
pub(crate) struct ChainedMethod {
    pub ident: Ident,
    pub args: Vec<Expr>,
}

/// Helper type for text/regex matcher args that may be either:
/// - a string literal, or
/// - an identifier (typically a constant).
pub(crate) enum IdentOrString {
    Ident(Ident),
    String(String),
}

impl FromInput for RuleExpr {
    fn from_input(input: &Input) -> zyn::Result<Self> {
        // Locate the `#[rule(...)]` attribute payload on the annotated item.
        let attr = input
            .attrs()
            .iter()
            .find(|a| a.path().is_ident("rule"))
            .ok_or_else(|| {
                zyn::syn::Error::new(zyn::Span::call_site(), "missing #[rule(...)] attribute")
            })?;

        Ok(attr.parse_args::<RuleExpr>()?)
    }
}

impl Parse for RuleExpr {
    fn parse(input: zyn::syn::parse::ParseStream) -> zyn::syn::Result<Self> {
        if input.peek(Ident) {
            // `#[rule(MyStructRule)]`
            Ok(Self::Struct(input.parse()?))
        } else if input.peek(Token![fn]) {
            // `#[rule(fn = make_rule)]`
            let _: Token![fn] = input.parse()?;
            let _: Token![=] = input.parse()?;
            Ok(Self::FnRef(input.parse()?))
        } else if input.peek(zyn::syn::token::Brace) {
            // `#[rule({ ...dsl... })]`
            let content;
            braced!(content in input);
            Ok(Self::Dsl(content.parse()?))
        } else {
            Err(zyn::syn::Error::new(
                input.span(),
                "Expected either a struct ident, a function reference (fn = ...), or a DSL expression!",
            ))
        }
    }
}

impl RuleExpr {
    /// Lowers high-level `RuleExpr` variants into concrete token output
    /// that produces `Box<dyn Rule + Send + Sync>`.
    pub fn to_tokens(&self) -> zyn::TokenStream {
        match self {
            Self::Struct(ident) => zyn::zyn! { ::std::boxed::Box::new({{ident}}::new()) },
            Self::FnRef(path) => zyn::zyn! { ::std::boxed::Box::new({{path}}()) },
            Self::Dsl(node) => zyn::zyn! { ::std::boxed::Box::new({{node.to_tokens()}}) },
        }
        .into()
    }
}

impl Parse for DslNode {
    fn parse(input: zyn::syn::parse::ParseStream) -> zyn::syn::Result<Self> {
        // Entry point: parse using lowest precedence rule (`then`).
        Self::parse_then(input)
    }
}

impl DslNode {
    /// Lowest precedence parser (`->`).
    ///
    /// This makes `a | b -> c` parse as `(a | b) -> c`.
    fn parse_then(input: zyn::syn::parse::ParseStream) -> syn::Result<Self> {
        let mut node = Self::parse_or(input)?;

        while input.peek(Token![->]) {
            input.parse::<Token![->]>()?;
            let right = Self::parse_or(input)?;
            node = DslNode::Then(Box::new(node), Box::new(right));
        }

        Ok(node)
    }

    /// OR parser (`|`) above `then`.
    fn parse_or(input: zyn::syn::parse::ParseStream) -> syn::Result<Self> {
        let mut nodes = vec![Self::parse_and(input)?];

        while input.peek(Token![|]) {
            input.parse::<Token![|]>()?;
            nodes.push(Self::parse_and(input)?);
        }

        if nodes.len() == 1 {
            Ok(nodes.pop().unwrap())
        } else {
            Ok(DslNode::Or(nodes))
        }
    }

    /// AND parser (`&`) above OR.
    fn parse_and(input: zyn::syn::parse::ParseStream) -> syn::Result<Self> {
        let mut nodes = vec![Self::parse_not(input)?];

        while input.peek(Token![&]) {
            input.parse::<Token![&]>()?;
            nodes.push(Self::parse_not(input)?);
        }

        if nodes.len() == 1 {
            Ok(nodes.pop().unwrap())
        } else {
            Ok(DslNode::And(nodes))
        }
    }

    /// Unary NOT parser (`!`) above AND.
    fn parse_not(input: zyn::syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![!]) {
            input.parse::<Token![!]>()?;
            let inner = Self::parse_base(input)?;
            Ok(DslNode::Not(Box::new(inner)))
        } else {
            Self::parse_base(input)
        }
    }

    /// Base parser:
    /// - grouped expression: `( ... )`
    /// - method chain terminal: `starts_with("x").wait(1)`
    fn parse_base(input: zyn::syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);

            // Parse a complete expression inside parentheses by restarting
            // from lowest precedence.
            let inner_node = DslNode::parse_then(&content)?;
            return Ok(inner_node);
        }

        let chain = MethodChain::parse(input)?;
        Ok(DslNode::MethodChain(chain))
    }

    /// Produce a fully built `Rule` expression.
    ///
    /// Convention:
    /// - `to_tokens_inner()` returns a `RuleBuilder` expression.
    /// - `to_tokens()` returns a final built `Rule`.
    ///
    /// This split avoids accidental double-builds while keeping composition ergonomic.
    fn to_tokens(&self) -> zyn::TokenStream {
        match self {
            Self::And(nodes) => {
                if nodes.is_empty() {
                    return zyn::zyn! {panic!("AND operator requires at least one operand")}.into();
                }
                let first = &nodes[0];
                let rest = &nodes[1..];
                zyn::zyn! {
                    {{ first.to_tokens_inner() }}
                    @for (node in rest.iter()) {
                        .and({{ node.to_tokens_inner() }}.build())
                    }.build()
                }
            }
            Self::Or(nodes) => {
                if nodes.is_empty() {
                    return zyn::zyn! {panic!("OR operator requires at least one operand")}.into();
                }
                let first = &nodes[0];
                let rest = &nodes[1..];
                zyn::zyn! {
                    {{ first.to_tokens_inner() }}
                    @for (node in rest.iter()) {
                        .or({{ node.to_tokens_inner() }}.build())
                    }.build()
                }
            }
            Self::Then(node1, node2) => {
                zyn::zyn! {
                    {{ node1.to_tokens_inner() }}.then({{ node2.to_tokens_inner() }}.build()).build()
                }
            }
            Self::Not(node) => {
                zyn::zyn! {
                    {{ node.to_tokens_inner() }}.not().build()
                }
            }
            Self::MethodChain(node) => {
                zyn::zyn! {
                    {{ node.to_tokens() }}.build()
                }
            }
        }
        .into()
    }

    /// Produce an *inner* `RuleBuilder` expression (not final `Rule`).
    ///
    /// This is used when the current node is nested under a parent combinator.
    /// Parent combinators call `.build()` on child builder expressions where required.
    fn to_tokens_inner(&self) -> zyn::TokenStream {
        match self {
            Self::And(nodes) => {
                if nodes.is_empty() {
                    return zyn::zyn! {panic!("AND operator requires at least one operand")}.into();
                }
                let first = &nodes[0];
                let rest = &nodes[1..];
                zyn::zyn! {
                    {{ first.to_tokens_inner() }}
                    @for (node in rest.iter()) {
                        .and({{ node.to_tokens_inner() }}.build())
                    }
                }
                .into()
            }
            Self::Or(nodes) => {
                if nodes.is_empty() {
                    return zyn::zyn! {panic!("OR operator requires at least one operand")}.into();
                }
                let first = &nodes[0];
                let rest = &nodes[1..];
                zyn::zyn! {
                    {{ first.to_tokens_inner() }}
                    @for (node in rest.iter()) {
                        .or({{ node.to_tokens_inner() }}.build())
                    }
                }
                .into()
            }
            Self::Then(node1, node2) => zyn::zyn! {
                {{ node1.to_tokens_inner() }}.then({{ node2.to_tokens_inner() }}.build())
            }
            .into(),
            Self::Not(node) => zyn::zyn! {
                {{ node.to_tokens_inner() }}.not()
            }
            .into(),
            Self::MethodChain(node) => zyn::zyn! {
                {{ node.to_tokens() }}
            }
            .into(),
        }
    }
}

impl Parse for MethodChain {
    fn parse(input: zyn::syn::parse::ParseStream) -> zyn::syn::Result<Self> {
        let base: MatcherCall = input.parse()?;
        let mut methods: Vec<ChainedMethod> = Vec::new();

        while input.peek(Token![.]) {
            let _: Token![.] = input.parse()?;
            methods.push(input.parse()?);
        }

        Ok(Self { base, methods })
    }
}

impl MethodChain {
    /// Convert matcher + chained methods into a `RuleBuilder` expression.
    ///
    /// Note:
    /// - `ChainedMethod::to_tokens()` returns `ident(args...)` (without leading dot),
    /// - this layer prefixes the dot while chaining (`.{{ method.to_tokens() }}`).
    fn to_tokens(&self) -> zyn::TokenStream {
        zyn::zyn! {
            ::binary_options_tools_core::rules::RuleBuilder::{{ self.base.to_tokens() }}
            @for (method in self.methods.iter()) {
                .{{ method.to_tokens() }}
            }
        }
        .into()
    }
}

impl Parse for MatcherCall {
    fn parse(input: zyn::syn::parse::ParseStream) -> zyn::syn::Result<Self> {
        if input.peek(Ident) {
            let ident: Ident = input.parse()?;
            if input.peek(Paren) {
                let content;
                let _ = parenthesized!(content in input);
                match ident.to_string().as_str() {
                    "any" => {
                        if content.is_empty() {
                            Ok(Self::Any)
                        } else {
                            Err(zyn::syn::Error::new(
                                ident.span(),
                                "Matcher 'any' does not take any arguments",
                            ))
                        }
                    }
                    "never" => {
                        if content.is_empty() {
                            Ok(Self::Never)
                        } else {
                            Err(zyn::syn::Error::new(
                                ident.span(),
                                "Matcher 'never' does not take any arguments",
                            ))
                        }
                    }
                    "exact" => Ok(Self::Exact(content.parse()?)),
                    "starts_with" => Ok(Self::StartsWith(content.parse()?)),
                    "ends_with" => Ok(Self::EndsWith(content.parse()?)),
                    "contains" => Ok(Self::Contains(content.parse()?)),
                    "regex" => Ok(Self::Regex(content.parse()?)),

                    "binary_exact" => Ok(Self::BinaryExact(content.parse()?)),
                    "binary_starts_with" => Ok(Self::BinaryStartsWith(content.parse()?)),
                    "binary_ends_with" => Ok(Self::BinaryEndsWith(content.parse()?)),
                    "binary_contains" => Ok(Self::BinaryContains(content.parse()?)),

                    "message_type" => Ok(Self::MessageType(content.parse()?)),
                    "json_schema" => Ok(Self::JsonSchema(content.parse()?)),

                    "custom" => Ok(Self::Custom(content.parse()?)),

                    _ => Err(zyn::syn::Error::new(
                        ident.span(),
                        format!("Unknown matcher '{}'", ident),
                    )),
                }
            } else {
                Err(zyn::syn::Error::new(
                    ident.span(),
                    format!("Expected parenthesis after matcher '{}'", ident),
                ))
            }
        } else {
            Err(zyn::syn::Error::new(
                input.span(),
                "Expected an ident for matcher!",
            ))
        }
    }
}

impl MatcherCall {
    /// Convert base matcher AST node to `RuleBuilder::<matcher>(...)`.
    fn to_tokens(&self) -> zyn::TokenStream {
        match self {
            Self::Any => zyn::zyn! { any() }.into(),
            Self::Never => zyn::zyn! { never() }.into(),
            Self::Exact(s) => zyn::zyn! { text_exact({{s.to_tokens()}}) }.into(),
            Self::StartsWith(s) => zyn::zyn! { text_starts_with({{s.to_tokens()}}) }.into(),
            Self::EndsWith(s) => zyn::zyn! { text_ends_with({{s.to_tokens()}}) }.into(),
            Self::Contains(s) => zyn::zyn! { text_contains({{s.to_tokens()}}) }.into(),
            Self::Regex(s) => zyn::zyn! { text_regex({{s.to_tokens()}}) }.into(),

            Self::BinaryExact(expr) => zyn::zyn! { binary_exact({{expr}}) }.into(),
            Self::BinaryStartsWith(expr) => zyn::zyn! { binary_starts_with({{expr}}) }.into(),
            Self::BinaryEndsWith(expr) => zyn::zyn! { binary_ends_with({{expr}}) }.into(),
            Self::BinaryContains(expr) => zyn::zyn! { binary_contains({{expr}}) }.into(),

            Self::MessageType(ident) => zyn::zyn! { message_type( ::binary_options_tools_core::rules::MessageType::{{ident}}) }.into(),
            Self::JsonSchema(ty) => zyn::zyn! { json_schema::<{{ty}}>() }.into(),

            Self::Custom(closure) => zyn::zyn! { custom({{closure}}) }.into(),
        }
    }
}

impl Parse for ChainedMethod {
    fn parse(input: zyn::syn::parse::ParseStream) -> zyn::syn::Result<Self> {
        if input.peek(Ident) {
            let ident: Ident = input.parse()?;
            if input.peek(Paren) {
                let content;
                parenthesized!(content in input);
                let mut args: Vec<Expr> = Vec::new();

                // Parse comma-separated expressions from the method argument list.
                if !content.is_empty() {
                    loop {
                        match content.parse::<Expr>() {
                            Ok(expr) => args.push(expr),
                            Err(_) => {
                                break;
                            }
                        }

                        if content.peek(Token![,]) {
                            content.parse::<Token![,]>()?;
                        } else {
                            break;
                        }
                    }
                }

                return Ok(Self { ident, args });
            }

            return Err(zyn::syn::Error::new(
                input.span(),
                format!("Expected parenthesis after ident '{}'", ident),
            ));
        }

        Err(zyn::syn::Error::new(input.span(), "Expected ident!"))
    }
}

impl ChainedMethod {
    /// Render a chained method segment as `method(arg1, arg2, ...)`.
    ///
    /// Dot prefix is intentionally *not* included here; it is added by
    /// `MethodChain::to_tokens()` while chaining.
    fn to_tokens(&self) -> zyn::TokenStream {
        let ident = &self.ident;
        let args = &self.args;
        zyn::zyn! {
            {{ ident }} (
                @for (arg in args) {
                    {{ arg }},
                }
            )
        }
        .into()
    }
}

impl Parse for IdentOrString {
    fn parse(input: zyn::syn::parse::ParseStream) -> zyn::syn::Result<Self> {
        if input.peek(Ident) {
            Ok(Self::Ident(input.parse()?))
        } else if input.peek(LitStr) {
            Ok(Self::String(input.parse::<LitStr>()?.value()))
        } else {
            Err(zyn::syn::Error::new(
                input.span(),
                "Expected either an identifier or a string literal",
            ))
        }
    }
}

impl IdentOrString {
    /// Render either:
    /// - an identifier token (constant/path segment), or
    /// - a string literal value.
    fn to_tokens(&self) -> zyn::TokenStream {
        match self {
            Self::Ident(ident) => zyn::zyn! { {{ident}} }.into(),
            Self::String(s) => zyn::zyn! { {{s}} }.into(),
        }
    }
}
