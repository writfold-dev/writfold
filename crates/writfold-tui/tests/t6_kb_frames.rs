//! Knowledge-base mode at the plain level: the status view, the stale
//! list and a cited source's inspector, at the sizes the guide shows.
mod support;

use support::TestResult;
use support::kb_app::{honest, kb_app, press};
use support::kb_world::atlas;
use support::projection::{golden, normalize, render, symbols};

#[test]
fn plain_kb_goldens() -> TestResult {
    let w = atlas("t6-kb-frames")?;
    for (name, width, height, keys) in [
        ("t6-kb-status-136x38.txt", 136, 38, "2"),
        ("t6-kb-stale-136x38.txt", 136, 38, "5jjjjj"),
        ("t6-kb-stale-inspector-80x24.txt", 80, 24, "5jjjjjl"),
    ] {
        let mut app = kb_app(&w);
        press(&mut app, keys);
        let text = symbols(&render(&mut app, width, height)?);
        assert!(!text.contains(&w.base.display().to_string()), "{name}");
        honest(&app, &text)?;
        golden(name, &normalize(&w, &text)?)?;
    }
    Ok(())
}
