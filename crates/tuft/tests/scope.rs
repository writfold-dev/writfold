//! Scope semantics, including zero-directory globstars and byte prefixes.
use tuft::scope::{matches, prefix};

#[test]
fn literal_scopes_preserve_prefix_matching() {
    for (scope, path, expected) in [
        ("", "src/lib.rs", true),
        ("src", "srcfoo/file", true),
        ("src/", "srcfoo/file", false),
        ("src/", "src/lib.rs", true),
        ("src/lib.rs", "src/lib.rs.bak", true),
        ("src/[ab]", "src/[ab].rs", true),
    ] {
        assert_eq!(matches(scope, path), expected, "{scope:?} {path:?}");
        assert_eq!(prefix(scope), scope);
    }
}

#[test]
fn globs_match_whole_paths_and_directory_boundaries() {
    for (scope, path, expected) in [
        ("src/*.rs", "src/lib.rs", true),
        ("src/*.rs", "src/nested/lib.rs", false),
        ("src/*.rs", "src/lib.rs.bak", false),
        ("src/?.rs", "src/a.rs", true),
        ("src/?.rs", "src/ab.rs", false),
        ("src/?.rs", "src//.rs", false),
        ("src/**.rs", "src/nested/lib.rs", true),
        ("src/**/*.rs", "src/lib.rs", true),
        ("src/**/*.rs", "src/nested/deep/lib.rs", true),
        ("**/lib.rs", "lib.rs", true),
        ("**/lib.rs", "mylib.rs", false),
        ("**/lib.rs", "src/mylib.rs", false),
        ("**/lib.rs", "src/lib.rs", true),
        ("**/a/**/b", "a/b", true),
        ("**/a/**/b", "x/a/y/z/b", true),
        ("**", "a/b", true),
        ("*", "a/b", false),
        ("src/??.rs", "src/λ.rs", true),
    ] {
        assert_eq!(matches(scope, path), expected, "{scope:?} {path:?}");
        if expected {
            assert!(path.starts_with(prefix(scope)));
        }
    }
    assert_eq!(prefix("src/**/lib?.rs"), "src/");
}

// Deliberately simple reference semantics, only over tiny test inputs.
fn glob(pattern: &[u8], path: &[u8]) -> bool {
    if pattern.starts_with(b"**/") {
        return glob(&pattern[3..], path)
            || path
                .iter()
                .enumerate()
                .any(|(i, &b)| b == b'/' && glob(&pattern[3..], &path[i + 1..]));
    }
    if pattern.starts_with(b"**") {
        return (0..=path.len()).any(|i| glob(&pattern[2..], &path[i..]));
    }
    match pattern.first() {
        None => path.is_empty(),
        Some(b'*') => (0..=path.iter().position(|&b| b == b'/').unwrap_or(path.len()))
            .any(|i| glob(&pattern[1..], &path[i..])),
        Some(b'?') => path.first().is_some_and(|&b| b != b'/') && glob(&pattern[1..], &path[1..]),
        Some(literal) => path.first() == Some(literal) && glob(&pattern[1..], &path[1..]),
    }
}

#[test]
fn bounded_path_space_agrees_with_reference_glob_semantics() {
    let patterns = [
        "*",
        "?",
        "**",
        "**/a",
        "a/**/b",
        "*/?",
        "a*b",
        "**/*a",
        "a/**",
        "**/a/**/b",
    ];
    for length in 0..=6 {
        for mut number in 0..3usize.pow(length) {
            let mut path = String::new();
            for _ in 0..length {
                path.push(['a', 'b', '/'][number % 3]);
                number /= 3;
            }
            for pattern in patterns {
                assert_eq!(
                    matches(pattern, &path),
                    glob(pattern.as_bytes(), path.as_bytes()),
                    "{pattern:?} {path:?}"
                );
            }
        }
    }
}
