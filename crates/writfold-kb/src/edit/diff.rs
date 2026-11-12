//! The preview: a unified diff built from a replay's exact splices. It is a
//! view for people and agents; the after bytes (and their blob ID) are what
//! the edit writes and the receipt records. Each changed block covers the
//! original lines its splices touch, trimmed to the lines that differ, with
//! three lines of context; blocks whose context meets share one hunk.

const CONTEXT: usize = 3;

/// One splice in original coordinates: `[lo, hi)` becomes `inserted`.
pub struct Splice<'a> {
    pub lo: usize,
    pub hi: usize,
    pub inserted: &'a [u8],
}

/// Original lines `[first, last)` become `new` (whole lines).
struct Block<'a> {
    first: usize,
    last: usize,
    new: Vec<&'a [u8]>,
}

fn lines(bytes: &[u8]) -> Vec<&[u8]> {
    bytes.split_inclusive(|b| *b == b'\n').collect()
}

/// The original lines each splice touches, merged where they meet, then
/// rebuilt with the splices applied and trimmed to the lines that differ.
fn blocks<'a>(before: &[u8], after: &'a [u8], splices: &[Splice<'_>]) -> Vec<Block<'a>> {
    let old = lines(before);
    let mut starts = Vec::with_capacity(old.len() + 1);
    let mut at = 0;
    for line in &old {
        starts.push(at);
        at += line.len();
    }
    let line_of = |byte: usize| starts.partition_point(|s| *s <= byte).saturating_sub(1);
    let ends_clean = before.is_empty() || before.ends_with(b"\n");
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for s in splices {
        let first = if s.lo == before.len() && ends_clean {
            old.len()
        } else {
            line_of(s.lo)
        };
        let last = if s.hi > s.lo {
            line_of(s.hi - 1) + 1
        } else {
            (first + 1).min(old.len())
        };
        match spans.last_mut() {
            Some(span) if first <= span.1 => span.1 = span.1.max(last),
            _ => spans.push((first, last.max(first))),
        }
    }
    let new = lines(after);
    let mut out = Vec::new();
    let mut shift: isize = 0;
    let mut spliced = 0;
    for (first, last) in spans {
        let lo = starts.get(first).copied().unwrap_or(before.len());
        let hi = starts.get(last).copied().unwrap_or(before.len());
        let mut grown = 0isize;
        while let Some(s) = splices.get(spliced).filter(|s| s.hi <= hi && s.lo >= lo) {
            grown += s.inserted.len() as isize - (s.hi - s.lo) as isize;
            spliced += 1;
        }
        let new_lo = (lo as isize + shift) as usize;
        let new_hi = (hi as isize + shift + grown) as usize;
        shift += grown;
        let replaced: Vec<&[u8]> = new
            .iter()
            .scan(0usize, |at, line| {
                let start = *at;
                *at += line.len();
                Some((start, *line))
            })
            .filter(|(start, _)| *start >= new_lo && *start < new_hi)
            .map(|(_, line)| line)
            .collect();
        out.push(trim(&old, first, last, replaced));
    }
    out
}

/// Drop the leading and trailing lines a block leaves unchanged.
fn trim<'a>(old: &[&[u8]], mut first: usize, mut last: usize, mut new: Vec<&'a [u8]>) -> Block<'a> {
    while first < last && new.first().is_some_and(|l| *l == old[first]) {
        new.remove(0);
        first += 1;
    }
    while first < last && new.last().is_some_and(|l| *l == old[last - 1]) {
        new.pop();
        last -= 1;
    }
    Block { first, last, new }
}

fn push_line(out: &mut String, mark: char, line: &[u8]) {
    out.push(mark);
    out.push_str(&String::from_utf8_lossy(line));
    if !line.ends_with(b"\n") {
        out.push_str("\n\\ No newline at end of file\n");
    }
}

fn range(start: usize, count: usize) -> String {
    let shown = if count == 0 { start } else { start + 1 };
    if count == 1 {
        shown.to_string()
    } else {
        format!("{shown},{count}")
    }
}

/// The unified diff of one file, or an empty string when nothing changed.
pub fn unified(path: &str, before: &[u8], after: &[u8], splices: &[Splice<'_>]) -> String {
    let old = lines(before);
    let found: Vec<Block<'_>> = blocks(before, after, splices)
        .into_iter()
        .filter(|b| b.first < b.last || !b.new.is_empty())
        .collect();
    if found.is_empty() {
        return String::new();
    }
    let mut out = format!("--- a/{path}\n+++ b/{path}\n");
    let mut shift: isize = 0;
    let mut index = 0;
    while index < found.len() {
        let mut end = index;
        while end + 1 < found.len() && found[end + 1].first <= found[end].last + 2 * CONTEXT {
            end += 1;
        }
        let from = found[index].first.saturating_sub(CONTEXT);
        let to = (found[end].last + CONTEXT).min(old.len());
        let mut body = String::new();
        let (mut removed, mut added, mut at) = (0usize, 0usize, from);
        for block in &found[index..=end] {
            for line in &old[at..block.first] {
                push_line(&mut body, ' ', line);
            }
            for line in &old[block.first..block.last] {
                push_line(&mut body, '-', line);
            }
            for line in &block.new {
                push_line(&mut body, '+', line);
            }
            removed += block.last - block.first;
            added += block.new.len();
            at = block.last;
        }
        for line in &old[at..to] {
            push_line(&mut body, ' ', line);
        }
        let kept = to - from - removed;
        let new_from = (from as isize + shift) as usize;
        out.push_str(&format!(
            "@@ -{} +{} @@\n{body}",
            range(from, kept + removed),
            range(new_from, kept + added)
        ));
        shift += added as isize - removed as isize;
        index = end + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Splice, unified};

    fn apply(before: &[u8], splices: &[Splice<'_>]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut at = 0;
        for s in splices {
            out.extend_from_slice(&before[at..s.lo]);
            out.extend_from_slice(s.inserted);
            at = s.hi;
        }
        out.extend_from_slice(&before[at..]);
        out
    }

    #[test]
    fn hunks_context_and_end_of_file() {
        let before = b"a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\n";
        let splices = [
            Splice {
                lo: 2,
                hi: 3,
                inserted: b"B",
            },
            Splice {
                lo: 26,
                hi: 27,
                inserted: b"N\nO",
            },
        ];
        let after = apply(before, &splices);
        let diff = unified("x.txt", before, &after, &splices);
        assert_eq!(
            diff,
            "--- a/x.txt\n+++ b/x.txt\n@@ -1,5 +1,5 @@\n a\n-b\n+B\n c\n d\n e\n@@ -11,4 +11,5 @@\n k\n l\n m\n-n\n+N\n+O\n"
        );
        let before = b"one\ntwo";
        let splices = [Splice {
            lo: 7,
            hi: 7,
            inserted: b"!",
        }];
        let after = apply(before, &splices);
        assert_eq!(
            unified("y", before, &after, &splices),
            "--- a/y\n+++ b/y\n@@ -1,2 +1,2 @@\n one\n-two\n\\ No newline at end of file\n+two!\n\\ No newline at end of file\n"
        );
    }

    #[test]
    fn insertion_at_the_end_and_unchanged() {
        let before = b"a\nb\n";
        let splices = [Splice {
            lo: 4,
            hi: 4,
            inserted: b"c\n",
        }];
        let after = apply(before, &splices);
        assert_eq!(
            unified("z", before, &after, &splices),
            "--- a/z\n+++ b/z\n@@ -1,2 +1,3 @@\n a\n b\n+c\n"
        );
        assert_eq!(unified("z", before, before, &[]), "");
    }
}
