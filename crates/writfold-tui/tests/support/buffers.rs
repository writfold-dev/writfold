//! Normalize random receipt suffixes even when the inspector wraps a path.
use super::TestResult;
use ratatui::buffer::Buffer;

pub fn stable(buffer: &mut Buffer) -> TestResult {
    let size = buffer.area;
    let areas = writfold_tui::layout::focused(size.width, size.height, true, true);
    let Some(area) = areas.inspector else {
        return Ok(());
    };
    let mut text = String::new();
    let mut positions = Vec::new();
    for y in area.y + 1..area.bottom().saturating_sub(1) {
        for x in area.x + 1..area.right().saturating_sub(1) {
            for ch in buffer[(x, y)]
                .symbol()
                .chars()
                .filter(|c| c.is_ascii() && !c.is_whitespace())
            {
                text.push(ch);
                positions.push((x, y));
            }
        }
    }
    let pattern = regex::Regex::new(r"-\d{8}T\d{6}Z-([0-9a-f]{6})\.json")?;
    for matched in pattern
        .captures_iter(&text)
        .filter_map(|capture| capture.get(1))
    {
        for position in &positions[matched.range()] {
            buffer[*position].set_symbol("X");
        }
    }
    Ok(())
}
