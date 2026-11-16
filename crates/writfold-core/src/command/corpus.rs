//! The generated table: any printable text inside single quotes, inside
//! double quotes with its specials escaped, or as a here-document body
//! with an unused delimiter, after `echo` or `cat`, is never a command.
use super::{Invocation, invocations};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

fn all(line: &str) -> Vec<Invocation> {
    invocations(line, "/w/a", Some("/home/u"))
}

/// A fixed pseudo-random corpus: no new crate, the same strings each run.
fn corpus() -> Vec<String> {
    let mut seed: u64 = 0x5eed_2026_1115;
    let mut out: Vec<String> = [
        "git commit",
        "x; git commit",
        "$(git push)",
        "a\ngit commit\nb",
        "`git commit`",
        "' ; git commit ; '",
        "\" && git push",
        "\\\ngit commit",
        "EOF",
        "<<EOF\ngit push",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    for _ in 0..400 {
        let mut s = String::new();
        let len = (seed % 40) as usize;
        for _ in 0..len {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let pick = (seed >> 33) % 100;
            let c = match pick {
                0..=4 => '\n',
                5..=9 => ' ',
                _ => char::from(32 + ((seed >> 40) % 95) as u8),
            };
            s.push(c);
        }
        if seed.is_multiple_of(3) {
            s.push_str("\ngit commit -m x\n");
        }
        out.push(s);
    }
    out
}

#[test]
fn quoted_text_and_here_document_bodies_are_never_commands() {
    for s in corpus() {
        let printable: String = s.chars().filter(|c| *c != '\n').collect();
        let single = format!("echo '{}'", printable.replace('\'', ""));
        let mut escaped = String::new();
        for c in printable.chars() {
            if matches!(c, '"' | '\\' | '$' | '`') {
                escaped.push('\\');
            }
            escaped.push(c);
        }
        let double = format!("cat \"{escaped}\"");
        let body = format!("cat <<'WF_END_7'\n{s}\nWF_END_7");
        let dashed = format!("cat <<-WF_END_7 >f\n{s}\n\tWF_END_7");
        for line in [single, double, body, dashed] {
            let named: Vec<Invocation> = all(&line)
                .into_iter()
                .filter(|i| i.words.first().is_some_and(|w| w == "git"))
                .collect();
            assert!(named.is_empty(), "{line:?}: {named:?}");
        }
    }
}
