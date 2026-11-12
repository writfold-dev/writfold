//! Property test of the citation verdict (design section 8.3, "citations
//! resolve or report"): cite random lines of a random file, edit the file
//! at random, and classify the citation with the kernel in
//! `writfold-core` over the span search. Exactly one verdict results, and
//!   - fresh implies the cited lines are equal after normalisation;
//!   - moved implies the reported range carries the cited span digest;
//!   - no citation is fresh when its lines changed.
use writfold_core::verdict::{Located, Observed, Target, Verdict, observed};
use writfold_kb::stale::span::locate;
use writfold_sources::citations::normalize_span;

struct Rng(u64);
impl Rng {
    fn next(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % bound.max(1) as u64) as usize
    }
}

const WORDS: [&str; 8] = [
    "fn a()",
    "let x = 1;",
    "",
    "  }",
    "x += 2;",
    "return x;",
    "// note",
    "{",
];

fn span_of(lines: &[String], first: usize, last: usize) -> Option<String> {
    let window: Vec<&str> = lines
        .get(first - 1..last)?
        .iter()
        .map(String::as_str)
        .collect();
    Some(normalize_span(&window))
}

fn edit(rng: &mut Rng, lines: &mut Vec<String>) {
    for _ in 0..rng.next(4) {
        let at = rng.next(lines.len() + 1);
        match rng.next(4) {
            0 => lines.insert(at, WORDS[rng.next(WORDS.len())].to_owned()),
            1 if at < lines.len() => {
                lines.remove(at);
            }
            2 if at < lines.len() => lines[at] = format!("  {}  ", lines[at].trim()),
            _ if at < lines.len() => lines[at] = WORDS[rng.next(WORDS.len())].to_owned(),
            _ => {}
        }
    }
}

#[test]
fn citations_resolve_or_report() {
    let mut rng = Rng(0x5eed_c17e_0000_0001);
    let (mut fresh, mut moved, mut changed) = (0, 0, 0);
    for _ in 0..3000 {
        let mut lines: Vec<String> = (0..1 + rng.next(14))
            .map(|_| WORDS[rng.next(WORDS.len())].to_owned())
            .collect();
        let first = 1 + rng.next(lines.len());
        let last = first + rng.next(lines.len() - first + 1);
        let Some(span) = span_of(&lines, first, last) else {
            continue;
        };
        let before: String = lines.iter().map(|l| format!("{l}\n")).collect();
        edit(&mut rng, &mut lines);
        let after: String = lines.iter().map(|l| format!("{l}\n")).collect();
        let target = if after == before {
            Target::SameBlob
        } else {
            let (a, b) = (first as u32, last as u32);
            Target::OtherBlob(Some(locate(&after, a, b, &span)))
        };
        let verdict = observed(Observed::Code(target));
        let now = span_of(&lines, first, last);
        match (verdict, target) {
            (Verdict::Fresh, _) => {
                fresh += 1;
                assert_eq!(
                    now.as_deref(),
                    Some(span.as_str()),
                    "{before:?} -> {after:?}"
                );
            }
            (Verdict::Moved, Target::OtherBlob(Some(Located::At(a, b)))) => {
                moved += 1;
                assert_eq!(
                    span_of(&lines, a as usize, b as usize).as_deref(),
                    Some(span.as_str())
                );
                assert_eq!(b - a, (last - first) as u32);
            }
            (Verdict::Changed, _) => changed += 1,
            other => unreachable!("unexpected {other:?}"),
        }
        if now.as_deref() != Some(span.as_str()) {
            assert_ne!(verdict, Verdict::Fresh);
        }
    }
    assert!(
        fresh > 100 && moved > 50 && changed > 50,
        "{fresh} {moved} {changed}"
    );
}
