pub const DEFAULT: &str = r#"
[logging]
level = "debug"
format = "json"
"#;

#[cfg(test)]
pub const TEST: &str = r#"
[logging]
level = "debug"
format = "pretty"
"#;
