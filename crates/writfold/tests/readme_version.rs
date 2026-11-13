//! The README's pre-release line names the version this binary reports, so
//! a release cannot leave the front page describing an older one.
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
    Ok(())
}
