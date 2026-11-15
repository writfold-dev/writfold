//! The app in KB mode, key presses, and the honesty check for KB frames.
use super::{RUN, TestResult, World};

/// The app on the atlas world's run, switched to KB mode with `m`.
pub fn kb_app(w: &World) -> writfold_tui::app::App {
    let mut app = writfold_tui::app::App::new(w.source(), Some(RUN));
    app.key(writfold_tui::app::Key::Char('m'));
    app
}

/// Press each character of `keys`.
pub fn press(app: &mut writfold_tui::app::App, keys: &str) {
    for c in keys.chars() {
        app.key(writfold_tui::app::Key::Char(c));
    }
}

/// Forbidden claims: liveness, durations, percentages. A word the verbs
/// themselves printed (authored text such as "the flush stalled", a
/// recorded `active` status) is quoted, not claimed, and is allowed.
pub fn honest(app: &writfold_tui::app::App, text: &str) -> TestResult {
    let forbidden = regex::Regex::new(
        r"(?i)\b(running|working|active|idle|stalled|live|in\s+progress|elapsed|relative|ago)\b|\d+(?:\.\d+)?\s*%",
    )?;
    let quoted: String = app
        .kb_replies()
        .iter()
        .map(|reply| format!("{:?} {:?}", reply.rows, reply.notes))
        .collect::<String>()
        .to_lowercase();
    for found in forbidden.find_iter(text) {
        let word = found.as_str().to_lowercase();
        if word.contains('%') || !quoted.contains(&word) {
            return Err(format!("forbidden {word:?} in frame: {text}").into());
        }
    }
    Ok(())
}
