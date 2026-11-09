//! Grammar golden cases. Every `cases/valid/*.tuft` compiles to the checked
//! pipeline in its `.expected`; every `cases/invalid/*.tuft` is refused with
//! exactly the diagnostics in its `.expected`. `TUFT_BLESS=1` rewrites the
//! expectations (review the diff before committing).
use std::fs;
use std::path::{Path, PathBuf};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn cases(kind: &str) -> Result<Vec<PathBuf>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cases")
        .join(kind);
    let mut paths: Vec<_> = fs::read_dir(dir)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    paths.retain(|p| p.extension().is_some_and(|e| e == "tuft"));
    paths.sort();
    Ok(paths)
}

fn summary(plan: &tuft::Plan) -> String {
    let (matcher, pattern) = plan.find.matcher.describe();
    format!(
        "find @{} {matcher} {:?} in {:?} at {} positions={} candidates={}\nretain {:?}\nproject {:?}\nemit {:?}\n",
        plan.find.binding,
        pattern,
        plan.find.scope,
        plan.find.commit,
        plan.find.budget.positions(),
        plan.find.budget.candidates(),
        plan.retain,
        plan.view,
        plan.emit
    )
}

fn compare(path: &Path, actual: &str) -> Result {
    let expected_path = path.with_extension("expected");
    if std::env::var_os("TUFT_BLESS").is_some() {
        fs::write(&expected_path, actual)?;
        return Ok(());
    }
    let expected = fs::read_to_string(&expected_path)
        .map_err(|e| format!("{}: {e}", expected_path.display()))?;
    if expected != actual {
        return Err(format!(
            "{}\n--- expected\n{expected}\n--- actual\n{actual}",
            path.display()
        )
        .into());
    }
    Ok(())
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[test]
fn valid_programs_compile_to_their_pipeline() -> Result {
    let paths = cases("valid")?;
    assert!(paths.len() >= 3);
    for path in paths {
        let source = fs::read(&path)?;
        let plan = tuft::compile(&source).map_err(|d| {
            format!(
                "{}: {}",
                path.display(),
                tuft::diag::render_all(&d, &source, &name(&path))
            )
        })?;
        assert_eq!(plan.program, tuft::digest_hex(&source));
        compare(&path, &summary(&plan))?;
    }
    Ok(())
}

#[test]
fn invalid_programs_refuse_with_exact_diagnostics() -> Result {
    let paths = cases("invalid")?;
    assert!(paths.len() >= 25);
    for path in paths {
        let source = fs::read(&path)?;
        let diagnostics = match tuft::compile(&source) {
            Ok(_) => return Err(format!("{} compiled", path.display()).into()),
            Err(diagnostics) => diagnostics,
        };
        let rendered = tuft::diag::render_all(&diagnostics, &source, &name(&path)) + "\n";
        compare(&path, &rendered)?;
    }
    Ok(())
}

#[test]
fn deferred_operators_refuse_by_type() {
    let commit = "0123456789abcdef0123456789abcdef01234567";
    let cases = [
        (
            format!(
                "@h := find similar \"x\" at \"{commit}\";\n@v := project @h [path];\nemit @v;\n"
            ),
            tuft::Deferred::Similar,
        ),
        (
            format!(
                "@h := find text \"x\" at \"{commit}\";\n@r := rank @h by \"y\";\n@v := project @r [path];\nemit @v;\n"
            ),
            tuft::Deferred::Rank,
        ),
        (
            format!(
                "@h := find text \"x\" at \"{commit}\";\n@r := follow @h via \"y\";\n@v := project @r [path];\nemit @v;\n"
            ),
            tuft::Deferred::Follow,
        ),
    ];
    for (source, which) in cases {
        let codes: Vec<_> = tuft::compile(source.as_bytes())
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.code)
            .collect();
        assert_eq!(codes, [tuft::Code::Unsupported(which)], "{source}");
    }
}
