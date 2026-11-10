use super::SpliceryExport;
use serde_json::json;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);

fn copy_directory(source: &Path, destination: &Path) -> Result {
    fs::create_dir(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let to = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_directory(&entry.path(), &to)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), to)?;
        } else {
            return Err("unexpected fixture source kind".into());
        }
    }
    Ok(())
}

fn fixture() -> Result<PathBuf> {
    let root = std::env::temp_dir().join(format!(
        "writfold-vendor-source-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root)?;
    fs::create_dir(root.join("vendor"))?;
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/splicery");
    copy_directory(&source, &root.join("vendor/splicery"))?;
    let export = SpliceryExport::admit(&root)?;
    let dependency = json!({"name":"splicery-domain","source":null,
        "path":root.join("vendor/splicery/crates/splicery-domain")});
    if !export.permits("writfold-sources", &dependency)
        || export.permits("writfold-domain", &dependency)
    {
        return Err("source-export control failed its dependency boundary".into());
    }
    Ok(root)
}

#[test]
fn identical_bytes_behind_a_file_symlink_are_not_the_pinned_export() -> Result {
    let root = fixture()?;
    let selected = root.join("vendor/splicery/LICENSE");
    let outside = root.join("retained-original-license");
    fs::rename(&selected, &outside)?;
    symlink(&outside, &selected)?;
    assert!(SpliceryExport::admit(&root).is_err());
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn identical_directory_alias_is_not_a_closed_source_export() -> Result {
    let root = fixture()?;
    let selected = root.join("vendor/splicery/crates/splicery-domain");
    let outside = root.join("retained-original-crate");
    fs::rename(&selected, &outside)?;
    symlink(&outside, &selected)?;
    assert!(SpliceryExport::admit(&root).is_err());
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn proved_splice_kernel_is_transitive_only_and_dependency_free() -> Result {
    let root = fixture()?;
    let export = SpliceryExport::admit(&root)?;
    let crate_dir = root.join("vendor/splicery/crates/splicery-splice");
    let direct = json!({"name":"splicery-splice","source":null,"path":crate_dir});
    assert!(!export.permits("writfold-sources", &direct));
    let manifest = crate_dir.join("Cargo.toml");
    let package = |dependencies: serde_json::Value| {
        json!({"name":"splicery-splice","source":null,
            "manifest_path":manifest,"dependencies":dependencies})
    };
    assert!(export.matches_package(&package(json!([]))));
    assert!(!export.matches_package(&package(json!([{"name":"regex"}]))));
    let kernel = json!({"name":"splicery-kernel","source":null,
        "manifest_path":root.join("vendor/splicery/crates/splicery-kernel/Cargo.toml"),
        "dependencies":[{"name":"splicery-splice"},{"name":"splicery-domain"}]});
    assert!(export.matches_package(&kernel));
    let mut legacy = kernel.clone();
    legacy["dependencies"] = json!([{"name":"splicery-domain"},{"name":"vstd"}]);
    assert!(!export.matches_package(&legacy));
    fs::remove_dir_all(root)?;
    Ok(())
}
