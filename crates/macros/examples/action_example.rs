#![allow(unused)]
// use binary_options_tools_macros::ActionImpl;
// // Define the ActionName trait (you'll need to import this in real usage)
// trait ActionName {
//     fn name(&self) -> &str;
// }

// // Example usage of the ActionImpl derive macro
// #[derive(ActionImpl)]
// #[action(name = "login")]
// struct LoginAction {
//     username: String,
//     password: String,
// }

// #[derive(ActionImpl)]
// #[action(name = "trade")]
// struct TradeAction {
//     asset: String,
//     amount: f64,
//     direction: String,
// }

// #[derive(ActionImpl)]
// #[action(name = "get_balance")]
// enum BalanceAction {
//     Real,
//     Demo,
// }


fn main() {
    // let login = LoginAction {
    //     username: "user".to_string(),
    //     password: "pass".to_string(),
    // };

    // let trade = TradeAction {
    //     asset: "EURUSD".to_string(),
    //     amount: 100.0,
    //     direction: "call".to_string(),
    // };

    // let balance = BalanceAction::Real;

    // // The macro automatically implements ActionName::name()
    // println!("Login action: {}", login.name());     // "login"
    // println!("Trade action: {}", trade.name());     // "trade"
    // println!("Balance action: {}", balance.name()); // "get_balance"
}
