use super::{Freshness, check::check, normalise, origin_in};

#[test]
fn urls_compare_by_host_and_path() {
    let same = [
        "git@Example.org:me/scratch.git",
        "ssh://git@example.org/me/scratch",
        "https://example.org/me/scratch.git/",
        "https://user@EXAMPLE.org/me/scratch",
    ];
    for url in same {
        assert_eq!(normalise(url), "example.org/me/scratch", "{url}");
    }
    assert_ne!(
        normalise("git@example.org:me/other.git"),
        normalise("git@example.org:me/scratch.git")
    );
    assert_eq!(normalise("/srv/git/scratch.git"), "/srv/git/scratch");
    assert_eq!(normalise("file:///srv/git/scratch"), "/srv/git/scratch");
}

#[test]
fn the_origin_is_read_from_its_own_section_only() {
    let config = "[core]\n\turl = nope\n[remote \"upstream\"]\n\turl = a\n[remote \"origin\"]\n\tfetch = x\n\turl = git@example.org:me/scratch.git\n";
    assert_eq!(
        origin_in(config).as_deref(),
        Some("git@example.org:me/scratch.git")
    );
    assert_eq!(origin_in("[remote \"upstream\"]\nurl = a\n"), None);
}

const GOOD: &str = "schema = \"writfold-store/1\"\nname = \"scratch\"\nurl = \"git@example.org:me/scratch.git\"\nrole = \"attached\"\n";

#[test]
fn a_store_is_pinned_unless_it_says_head() {
    let store = check(GOOD, "scratch", "s.toml").map_err(|r| crate::defs::why(&r));
    assert_eq!(store.as_ref().map(|s| s.freshness), Ok(Freshness::Pinned));
    assert_eq!(store.map(|s| s.paths.len()), Ok(0));
    let head = format!("{GOOD}freshness = \"head\"\npaths = [\"~/src/scratch\"]\n");
    let store = check(&head, "scratch", "s.toml").map_err(|r| crate::defs::why(&r));
    assert_eq!(store.as_ref().map(|s| s.freshness), Ok(Freshness::Head));
    assert_eq!(store.map(|s| s.paths), Ok(vec!["~/src/scratch".to_owned()]));
}

/// Each fault refuses the whole file: a misspelt key never silently
/// changes which rule judges evidence.
#[test]
fn a_faulty_declaration_is_refused() {
    let faults = [
        (
            GOOD.replace("writfold-store/1", "writfold-store/2"),
            "scratch",
        ),
        (GOOD.replace("attached", "curated"), "scratch"),
        (format!("{GOOD}freshness = \"fresh\"\n"), "scratch"),
        (format!("{GOOD}fressness = \"head\"\n"), "scratch"),
        (
            GOOD.replace("url = \"git@example.org:me/scratch.git\"\n", ""),
            "scratch",
        ),
        (GOOD.to_owned(), "other"),
    ];
    for (text, file) in faults {
        assert!(check(&text, file, "s.toml").is_err(), "{text}");
    }
}
