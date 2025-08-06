# ActionImpl Derive Macro

The `ActionImpl` derive macro automatically implements the `ActionName` trait for structs and enums. This is useful for the ExpertOptions API where different action types need to have a name identifier.

## Usage

1. **Import the macro and trait:**

```rust
use binary_options_tools_macros::ActionImpl;
use your_crate::expertoptions::action::ActionName;  // Import the trait
```

2. **Add the derive macro to your struct or enum:**

```rust
#[derive(ActionImpl)]
#[action = "your_action_name"]
struct YourAction {
    // your fields
}
```

## Examples

### Struct Example

```rust
use binary_options_tools_macros::ActionImpl;

#[derive(ActionImpl)]
#[action = "login"]
struct LoginAction {
    username: String,
    password: String,
}

// Generated implementation:
// impl ActionName for LoginAction {
//     fn name(&self) -> &str {
//         "login"
//     }
// }
```

### Enum Example

```rust
use binary_options_tools_macros::ActionImpl;

#[derive(ActionImpl)]
#[action = "get_balance"]
enum BalanceAction {
    Real,
    Demo,
}

// Generated implementation:
// impl ActionName for BalanceAction {
//     fn name(&self) -> &str {
//         "get_balance"
//     }
// }
```

### Multiple Actions

```rust
use binary_options_tools_macros::ActionImpl;

#[derive(ActionImpl)]
#[action = "trade"]
struct TradeAction {
    asset: String,
    amount: f64,
    direction: String,
}

#[derive(ActionImpl)]
#[action = "close_trade"]
struct CloseTradeAction {
    trade_id: u64,
}

#[derive(ActionImpl)]
#[action = "get_assets"]
struct GetAssetsAction;

// Usage:
fn main() {
    let trade = TradeAction {
        asset: "EURUSD".to_string(),
        amount: 100.0,
        direction: "call".to_string(),
    };

    let close = CloseTradeAction { trade_id: 12345 };
    let assets = GetAssetsAction;

    println!("Trade action: {}", trade.name());     // "trade"
    println!("Close action: {}", close.name());    // "close_trade"
    println!("Assets action: {}", assets.name());  // "get_assets"
}
```

## Requirements

- The `#[action = "action_name"]` attribute is required
- The action name must be a string literal
- The type must be a struct or enum

## Error Handling

The macro will produce compile-time errors if:

- The `#[action = "..."]` attribute is missing
- The action attribute doesn't have a string value
- The attribute format is incorrect

## Generated Code

For each type annotated with `#[derive(ActionImpl)]` and `#[action = "name"]`, the macro generates:

```rust
impl ActionName for YourType {
    fn name(&self) -> &str {
        "name"  // The value from the action attribute
    }
}
```

This allows you to call `.name()` on any instance of your action types to get their string identifier.
