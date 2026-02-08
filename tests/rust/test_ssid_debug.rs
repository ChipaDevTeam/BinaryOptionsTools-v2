use binary_options_tools::pocketoption::ssid::Ssid;

#[test]
fn test_ssid_redaction() {
    let ssid_json = r#"42["auth",{"session":"SECRET_SESSION","isDemo":1,"uid":123,"platform":1}]"#;
    let ssid = Ssid::parse(ssid_json).unwrap();
    let debug_str = format!("{:?}", ssid);
    assert!(debug_str.contains("REDACTED"));
    assert!(!debug_str.contains("SECRET_SESSION"));
    println!("SSID Debug: {}", debug_str);
}
