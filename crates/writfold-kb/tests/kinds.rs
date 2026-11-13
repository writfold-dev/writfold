//! Kind declarations (`.writfold/sources/<kind>.toml`): a default lifetime
//! for tokens without `ttl=`, the immutable rule, required fields, the
//! refusals, the verdict cache taking a change at the next `kb stale`, and
//! a property test against the kernel over the effective lifetime.
mod support;

use support::{NOW, TestResult, World, git, world};
use writfold_core::verdict::{Lifetime, Observed, lifetime, observed};
use writfold_kb::Verb;
use writfold_kb::stale::kinds::{self, Freshness, Kind, Kinds};
use writfold_kb::stale::{Cite, Verdict};
use writfold_sources::citations::{remove_field, replace_field};

/// Appended to the harbor status, so `brief harbor` shows its verdict.
const FLUSH_TOKEN: &str =
    "\n⟦cite kind=web url=https://example.invalid/wiki/flush seen=2026-10-01T00:00Z⟧\n";
/// A note of their own (outside `brief harbor`'s five cached rows).
const TOKENS: &str = "# Sources\n\n\
⟦cite kind=ticket id=HBR-7 board=ops seen=2026-10-01T00:00Z ttl=90d⟧\n\
⟦cite kind=ticket board=ops seen=2026-11-10T00:00Z⟧\n\
⟦cite kind=dataset ref=snap-2026-10⟧\n\
⟦cite kind=zeppelin note=hangar⟧\n\
⟦cite kind=zeppelin note=mast seen=2020-01-01⟧\n";

const WEB: &str =
    "schema = \"writfold-source/1\"\nkind = \"web\"\nfreshness = \"ttl\"\ndefault_ttl = \"30d\"\n";
const TICKET: &str = "schema = \"writfold-source/1\"\nkind = \"ticket\"\nfreshness = \"ttl\"\ndefault_ttl = \"3d\"\nrequired = [\"id\"]\ndescription = \"tracker items\"\n";
const DATASET: &str =
    "schema = \"writfold-source/1\"\nkind = \"dataset\"\nfreshness = \"immutable\"\n";

fn put(w: &World, path: &str, text: &str) -> TestResult {
    let file = w.kb.join(path);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, text)?;
    Ok(())
}

fn commit(w: &World, message: &str) -> TestResult {
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", message])?;
    Ok(())
}

/// The fixture with the tokens above.
fn sourced() -> TestResult<World> {
    let w = world(&[])?;
    let status = std::fs::read_to_string(w.kb.join("harbor/STATUS.md"))?;
    put(&w, "harbor/STATUS.md", &format!("{status}{FLUSH_TOKEN}"))?;
    put(&w, "lantern/sources.md", TOKENS)?;
    commit(&w, "sources")?;
    Ok(w)
}

fn declare(w: &World) -> TestResult {
    put(w, ".writfold/sources/web.toml", WEB)?;
    put(w, ".writfold/sources/ticket.toml", TICKET)?;
    put(w, ".writfold/sources/dataset.toml", DATASET)?;
    commit(w, "kinds")
}

fn stale(w: &World, offline: bool) -> TestResult<(String, u8)> {
    w.run(&w.request(Verb::Stale {
        scope: None,
        offline,
    }))
}

fn brief(w: &World) -> TestResult<String> {
    let verb = Verb::Brief {
        project: Some("harbor".into()),
        all: false,
    };
    Ok(w.run(&w.request(verb))?.0)
}

fn row<'a>(text: &'a str, target: &str) -> Option<&'a str> {
    text.lines()
        .find(|l| l.starts_with(&format!("cite {target} ")))
}

const FLUSH: &str = "web:https://example.invalid/wiki/flush";

#[test]
fn undeclared_kinds_are_unchanged() -> TestResult {
    let w = sourced()?;
    let (text, _) = stale(&w, false)?;
    // No ttl= and no declaration: fresh for ever, as before.
    assert_eq!(row(&text, FLUSH), None, "{text}");
    assert_eq!(row(&text, "ticket:HBR-7"), None, "{text}");
    assert_eq!(row(&text, "ticket:"), None, "{text}");
    // An unknown kind is never malformed: unverifiable without seen=, and
    // fresh for ever with it.
    let hangar = row(&text, "zeppelin:").ok_or(text.clone())?;
    assert!(
        hangar.contains("[unverifiable] at none missing seen=") && !hangar.contains("malformed"),
        "{hangar}"
    );
    assert_eq!(text.matches("cite zeppelin:").count(), 1, "{text}");
    let dataset = row(&text, "dataset:snap-2026-10").ok_or(text.clone())?;
    assert!(dataset.contains("missing seen="), "{dataset}");
    Ok(())
}

#[test]
fn declarations_take_effect_at_the_next_stale() -> TestResult {
    let w = sourced()?;
    stale(&w, false)?;
    declare(&w)?;
    // `brief` shows the cache, which only `kb stale` rewrites.
    let before = brief(&w)?;
    assert!(!before.contains(FLUSH), "{before}");
    let (text, exit) = stale(&w, false)?;
    assert_eq!(exit, 2, "{text}");
    // The declared default applies to a token without ttl=.
    let flush = row(&text, FLUSH).ok_or(text.clone())?;
    assert!(
        flush.contains("[expired] at seen:2026-10-01T00:00Z"),
        "{flush}"
    );
    // A token's own ttl= wins over the default (90d, not 3d).
    assert_eq!(row(&text, "ticket:HBR-7"), None, "{text}");
    // A missing required field is unverifiable and named.
    let missing = row(&text, "ticket:").ok_or(text.clone())?;
    assert!(
        missing.contains(
            "[unverifiable] at none missing id= (required by .writfold/sources/ticket.toml)"
        ),
        "{missing}"
    );
    // Immutable: fresh without seen=.
    assert_eq!(row(&text, "dataset:snap-2026-10"), None, "{text}");
    // Undeclared kinds are as before.
    assert!(row(&text, "zeppelin:").is_some(), "{text}");
    let after = brief(&w)?;
    let flagged = after
        .lines()
        .find(|l| l.contains(FLUSH))
        .ok_or(after.clone())?;
    assert!(flagged.ends_with("!expired"), "{flagged}");
    // Offline reports the declared default as due; immutable never.
    let (offline, _) = stale(&w, true)?;
    assert!(
        row(&offline, FLUSH).is_some_and(|r| r.contains("[due]")),
        "{offline}"
    );
    assert_eq!(row(&offline, "dataset:snap-2026-10"), None, "{offline}");
    // Changing the default changes the verdict at the next `kb stale`.
    put(&w, ".writfold/sources/web.toml", &WEB.replace("30d", "90d"))?;
    commit(&w, "longer")?;
    assert!(brief(&w)?.contains(FLUSH));
    let (text, _) = stale(&w, false)?;
    assert_eq!(row(&text, FLUSH), None, "{text}");
    assert!(!brief(&w)?.contains(FLUSH));
    Ok(())
}

#[test]
fn bad_declarations_are_refused_and_reported_by_check() -> TestResult {
    let refused = [
        (WEB.replace("web", "code"), "code", "cannot be declared"),
        (WEB.replace("web", "record"), "record", "cannot be declared"),
        (WEB.replace("web", "gate"), "gate", "cannot be declared"),
        (
            WEB.replace("web", "proposal"),
            "proposal",
            "cannot be declared",
        ),
        (
            format!("{DATASET}default_ttl = \"3d\"\n"),
            "dataset",
            "immutable",
        ),
        (WEB.replace("30d", "30m"), "web", "<N>d or <N>h"),
        (WEB.replace("30d", "none"), "web", "<N>d or <N>h"),
        (
            format!("{WEB}colour = \"blue\"\n"),
            "web",
            "unknown key colour",
        ),
        (WEB.replace("source/1", "source/2"), "web", "schema"),
        (WEB.to_owned(), "pages", "must match the file name"),
        (WEB.replace("\"ttl\"", "\"weekly\""), "web", "freshness"),
        (format!("{WEB}required = \"id\"\n"), "web", "array"),
    ];
    for (text, name, why) in &refused {
        let error = kinds::parse(text, name).err().unwrap_or_default();
        assert!(error.contains(why), "{name}: {error:?} lacks {why:?}");
    }
    let ticket = kinds::parse(TICKET, "ticket")?;
    assert_eq!(ticket.default_ttl.as_deref(), Some("3d"));
    assert_eq!(ticket.required, ["id"]);
    assert_eq!(ticket.description.as_deref(), Some("tracker items"));
    assert_eq!(
        kinds::parse(DATASET, "dataset")?.freshness,
        Freshness::Immutable
    );
    // In a KB: `stale` refuses, naming the file, and `check` reports it.
    let w = sourced()?;
    put(
        &w,
        ".writfold/sources/web.toml",
        &format!("{WEB}colour = \"blue\"\n"),
    )?;
    let request = w.request(Verb::Stale {
        scope: None,
        offline: false,
    });
    let error = writfold_kb::run(&request).err().ok_or("stale answered")?;
    assert_eq!(error.exit_code(), 3);
    assert_eq!(
        error.to_string(),
        "refused: .writfold/sources/web.toml: unknown key colour"
    );
    let (text, _) = w.run(&w.request(Verb::Check { scope: None }))?;
    let line = text
        .lines()
        .find(|l| l.starts_with("check stale "))
        .unwrap_or_default();
    assert!(
        line.starts_with(
            "check stale [refused] refused: .writfold/sources/web.toml: unknown key colour"
        ),
        "{text}"
    );
    Ok(())
}

#[test]
fn unknown_fields_round_trip_byte_for_byte() -> TestResult {
    let raw = "⟦cite kind=ticket id=HBR-7 board=ops x-team=quay  seen=2026-10-01T00:00Z⟧";
    assert_eq!(
        replace_field(raw, "seen", "2026-11-11T00:00Z")?,
        "⟦cite kind=ticket id=HBR-7 board=ops x-team=quay  seen=2026-11-11T00:00Z⟧"
    );
    assert_eq!(
        replace_field(raw, "id", "HBR-8")?,
        raw.replace("HBR-7", "HBR-8")
    );
    assert_eq!(
        remove_field(raw, "seen")?,
        "⟦cite kind=ticket id=HBR-7 board=ops x-team=quay⟧"
    );
    assert_eq!(remove_field(raw, "absent")?, raw);
    Ok(())
}

struct Rng(u64);
impl Rng {
    fn next(&mut self, bound: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % bound.max(1)
    }
}

/// A random `<N>d` or `<N>h`, with its length in seconds.
fn ttl(rng: &mut Rng) -> (String, i64) {
    let n = rng.next(60);
    let n_i = i64::try_from(n).unwrap_or_default();
    if rng.next(2) == 0 {
        (format!("{n}d"), n_i * 86_400)
    } else {
        (format!("{n}h"), n_i * 3600)
    }
}

#[test]
fn the_verdict_is_the_kernels_over_the_effective_lifetime() -> TestResult {
    let mut rng = Rng(0x5eed_0fc1_a55e_0005);
    let mut seen_verdicts = std::collections::BTreeSet::new();
    for _ in 0..4000 {
        let token = (rng.next(2) == 0).then(|| ttl(&mut rng));
        let default = (rng.next(3) != 0).then(|| ttl(&mut rng));
        let immutable = rng.next(5) == 0;
        let seen_at = NOW - i64::try_from(rng.next(120 * 86_400)).unwrap_or_default();
        let now = NOW + i64::try_from(rng.next(10 * 86_400)).unwrap_or_default();
        let seen = writfold_kb::time::stamp(seen_at);
        let declared = (immutable || default.is_some()).then(|| Kind {
            name: "pages".into(),
            freshness: if immutable {
                Freshness::Immutable
            } else {
                Freshness::Ttl
            },
            default_ttl: if immutable {
                None
            } else {
                default.clone().map(|d| d.0)
            },
            required: Vec::new(),
            description: None,
        });
        let cite = Cite {
            kind: Some("pages".into()),
            seen: Some(seen.clone()),
            ttl: token.clone().map(|t| t.0),
            fields: "{}".into(),
            ..Cite::default()
        };
        let got = kinds::judge(&Kinds::new(declared.clone()), &cite, "pages", now).verdict;
        let default_secs = declared.as_ref().and(default.map(|d| d.1));
        let fact = match lifetime(immutable, token.map(|t| t.1), default_secs) {
            Lifetime::Immutable => Observed::Immutable,
            Lifetime::Token(s) | Lifetime::Default(s) => Observed::Timed(Some(now - seen_at > s)),
            Lifetime::Never => Observed::Timed(Some(false)),
        };
        let want = match observed(fact) {
            writfold_core::verdict::Verdict::Fresh => Some(Verdict::Fresh),
            writfold_core::verdict::Verdict::Expired => Some(Verdict::Expired),
            _ => None,
        };
        assert_eq!(
            Some(&got),
            want.as_ref(),
            "seen={seen} now={now} cite={cite:?} {declared:?}"
        );
        seen_verdicts.insert(got.name());
    }
    assert_eq!(seen_verdicts.len(), 2, "{seen_verdicts:?}");
    Ok(())
}
