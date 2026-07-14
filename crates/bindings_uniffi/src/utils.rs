pub fn default_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(30)
}

pub fn format_error(err: impl std::fmt::Display) -> String {
    format!("BinaryOptionsToolsError: {}", err)
}
