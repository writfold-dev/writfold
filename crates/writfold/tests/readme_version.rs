//! The README's pre-release line and its manual download URL name the
//! version this binary reports, so a release cannot leave the front page
//! advertising one release and installing another (#49).
use std::error::Error;
use std::path::Path;

#[test]
fn readme_names_the_workspace_version() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let readme = std::fs::read_to_string(root.join("README.md"))?;
    let line = format!("**Pre-release (`v{}`).**", env!("CARGO_PKG_VERSION"));
    assert!(
        readme.contains(&line),
        "README.md should contain {line:?}; update it with the version"
    );
    let base = "https://github.com/writfold-dev/writfold/releases/download/v";
    let tags: Vec<&str> = (readme.match_indices(base))
        .map(|(at, _)| {
            let rest = &readme[at + base.len()..];
            rest.split(|c: char| c.is_whitespace() || c == '"' || c == '/')
                .next()
                .unwrap_or_default()
        })
        .collect();
    assert!(
        !tags.is_empty(),
        "README.md should give a manual download URL"
    );
    for tag in tags {
        assert_eq!(
            tag,
            env!("CARGO_PKG_VERSION"),
            "README.md's manual download installs v{tag}; update it with the version"
        );
    }
    Ok(())
}
