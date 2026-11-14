use super::{TestResult, World};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use writfold_kb::{Verb, gate::GateVerb, plan::PlanVerb};
use writfold_tui::app::{App, View};

pub fn verb(view: View, run: &str, project: Option<&str>) -> Verb {
    match view {
        View::Progress => Verb::Plan(PlanVerb::Status {
            run: Some(run.into()),
            repo: None,
            gates: true,
        }),
        View::Gates => Verb::Gate(GateVerb::Status {
            run: Some(run.into()),
            repo: None,
        }),
        View::Timeline => Verb::Timeline { target: run.into() },
        View::Guards => Verb::Guard(writfold_kb::guard::GuardVerb::Status),
        View::Inbox | View::Resume => Verb::Resume {
            run: Some(run.into()),
        },
        View::Stale => Verb::Stale {
            scope: project.map(str::to_owned),
            offline: false,
            cached: true,
        },
        View::Brief => Verb::Brief {
            project: project.map(str::to_owned),
            all: false,
        },
    }
}

pub fn normalize(w: &World, text: &str) -> TestResult<String> {
    let text = super::stable(text).replace(&w.base.display().to_string(), "<WORLD>");
    let pin = regex::Regex::new(r"kb=[0-9a-f]{12,64}")?;
    Ok(pin.replace_all(&text, "kb=XXXXXXXXXXXX").into_owned())
}

pub fn golden(name: &str, actual: &str) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(file, actual)?;
    } else {
        let expected = std::fs::read_to_string(file)?;
        assert_eq!(expected, actual, "golden {name}");
    }
    Ok(())
}

pub fn render(app: &mut App, width: u16, height: u16) -> TestResult<Buffer> {
    app.asked = "00:00:00Z".into();
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    terminal.draw(|frame| writfold_tui::view::draw(frame, app))?;
    Ok(terminal.backend().buffer().clone())
}

pub fn rows(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

pub fn symbols(buffer: &Buffer) -> String {
    let rows = rows(buffer);
    let text = rows
        .iter()
        .map(|r| r.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    format!("{text}\n")
}

pub fn snapshot(root: &Path) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
    fn visit(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) -> TestResult {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_name() == ".git" {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, out)?;
            } else {
                out.insert(path.strip_prefix(root)?.into(), std::fs::read(path)?);
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    if root.exists() {
        visit(root, root, &mut out)?;
    }
    Ok(out)
}

pub fn local_snapshot(w: &World) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
    let ws = writfold_kb::workspace::discover(Some(&w.kb), &w.kb)?;
    let state = writfold_kb::proposal::state_of(&ws).ok_or("local state")?;
    let mut out = snapshot(&state)?;
    // The index is derived; the binary test checks its cite_checks table separately.
    out.retain(|path, _| {
        !path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("kb-index-"))
    });
    Ok(out)
}

pub fn git_status(w: &World) -> TestResult<Vec<u8>> {
    let output = std::process::Command::new("git")
        .current_dir(&w.kb)
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(output.stdout)
}
