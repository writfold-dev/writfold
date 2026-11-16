//! The fenced `toml` blocks of a guide page, each with whether the line
//! before its fence (blank lines skipped) is the planned marker.

pub const PLANNED: &str = "<!-- planned -->";

pub struct Block {
    pub at: String,
    pub planned: bool,
    pub text: String,
}

/// The `toml` blocks of one page, each with whether the line before its
/// fence (blank lines skipped) is the planned marker.
pub fn blocks(page: &str, text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut open: Option<Block> = None;
    let mut before = "";
    for (n, line) in text.lines().enumerate() {
        match open.take() {
            Some(block) if line.trim_end() == "```" => {
                out.push(block);
                before = "";
            }
            Some(mut block) => {
                block.text.push_str(line);
                block.text.push('\n');
                open = Some(block);
            }
            None if line.trim_end() == "```toml" => {
                open = Some(Block {
                    at: format!("{page}.md:{}", n + 1),
                    planned: before == PLANNED,
                    text: String::new(),
                });
            }
            None => {
                if !line.trim().is_empty() {
                    before = line.trim();
                }
            }
        }
    }
    out
}

#[test]
fn a_planned_marker_is_read_as_planned() {
    let page = "<!-- planned -->\n```toml\nschema = \"x/9\"\n```\n\n```toml\n```\n";
    let marks: Vec<bool> = blocks("page", page).iter().map(|b| b.planned).collect();
    assert_eq!(marks, [true, false]);
}
