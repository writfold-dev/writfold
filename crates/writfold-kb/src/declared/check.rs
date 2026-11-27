//! The checks of a repository declaration ([`super`]): every fault of the
//! file, each at its position.
use super::{Declared, SCHEMA};
use crate::defs::{Check, Field};
use crate::gate::def_check::{description, schema};
use crate::stale::kinds::is_word;
use writfold_core::diag::Report;

const KEYS: [&str; 8] = [
    "schema",
    "name",
    "url",
    "mirrors",
    "aliases",
    "ref",
    "description",
    "provider",
];
const PROVIDERS: [&str; 1] = ["git"];

/// A repository name: words joined by `/` (`org/service`).
pub fn is_name(name: &str) -> bool {
    !name.is_empty() && name.split('/').all(is_word)
}

fn one_url(c: &mut Check<'_>, text: &str, span: writfold_core::diag::Span) -> bool {
    if text.trim().is_empty() || text.contains(['\n', '\r', ' ']) {
        let why = "a URL must be one remote URL with no spaces".to_owned();
        c.error("value.form", Some(span), why)
            .label("not a remote URL");
        return false;
    }
    true
}

fn strings(c: &mut Check<'_>, key: &str, url: bool) -> Option<Vec<String>> {
    let empty = crate::defs::Spanned {
        value: Vec::new(),
        span: Default::default(),
    };
    let list = c.list(key).or(empty)?.value;
    let mut ok = true;
    for item in &list {
        let good = match url {
            true => one_url(c, item.str(), item.span),
            false => is_name(item.str()),
        };
        if !good && !url {
            let why = format!("alias {:?} must be words joined by /", item.str());
            c.error("value.form", Some(item.span), why)
                .label("not a repository name");
        }
        ok &= good;
    }
    ok.then(|| list.into_iter().map(|p| p.value.text).collect())
}

fn provider(c: &mut Check<'_>) -> Option<()> {
    match c.text("provider") {
        Field::Absent => Some(()),
        Field::Bad => None,
        Field::Got(p) if p.str() == "git" => Some(()),
        Field::Got(p) => {
            let why = format!("unknown provider {:?}", p.str());
            c.unknown_value(p.span, why, "not a provider", p.str(), &PROVIDERS, '"');
            None
        }
    }
}

fn reference(c: &mut Check<'_>) -> Option<Option<String>> {
    match c.text("ref") {
        Field::Absent => Some(None),
        Field::Bad => None,
        Field::Got(r) => {
            let bad = r.str().is_empty()
                || r.str().contains(['\n', ' ', '~', '^', ':'])
                || r.str().starts_with('-');
            if bad {
                let why = format!("ref {:?} is not a branch name", r.str());
                c.error("value.form", Some(r.span), why)
                    .label("not a branch");
                return None;
            }
            Some(Some(r.value.text))
        }
    }
}

/// Check one declaration; `slug` is its file stem and `path` how rows show it.
pub fn check(text: &str, slug: &str, path: &str) -> Result<Declared, Report> {
    let mut c = Check::read(path, text, "a repository declaration");
    c.unknown_keys(&KEYS, &KEYS, "not a repository key");
    schema(&mut c, SCHEMA);
    let name = c.need_text("name", &format!("name = \"{slug}\""));
    let name = name.and_then(|n| {
        if is_name(n.str()) {
            Some(n.value.text)
        } else {
            let why = format!("name {:?} must be words joined by /", n.str());
            c.error("value.form", Some(n.span), why)
                .label("not a repository name");
            None
        }
    });
    let url = c
        .need_text("url", "url = \"<the remote every machine clones from>\"")
        .and_then(|u| one_url(&mut c, u.str(), u.span).then_some(u.value.text));
    let mirrors = strings(&mut c, "mirrors", true);
    let aliases = strings(&mut c, "aliases", false);
    let reference = reference(&mut c);
    let described = description(&mut c);
    let provided = provider(&mut c);
    let found = match (name, url, mirrors, aliases, reference, described, provided) {
        (
            Some(name),
            Some(url),
            Some(mirrors),
            Some(aliases),
            Some(reference),
            Some(description),
            Some(()),
        ) => Some(Declared {
            name,
            url,
            mirrors,
            aliases,
            reference,
            description,
            path: path.to_owned(),
        }),
        _ => None,
    };
    c.finish(found)
}

#[cfg(test)]
mod tests {
    use super::check;

    const GOOD: &str = "schema = \"writfold-repo/1\"\nname = \"org/service\"\nurl = \"https://example.org/org/service.git\"\nmirrors = [\"git@example.org:org/service.git\"]\naliases = [\"service\"]\nref = \"main\"\n";

    #[test]
    fn a_declaration_is_checked() {
        let d = check(GOOD, "service", "s.toml").map_err(|r| format!("{r:?}"));
        assert_eq!(d.as_ref().map(|d| d.name.as_str()), Ok("org/service"));
        assert_eq!(d.map(|d| d.reference), Ok(Some("main".into())));
        for bad in [
            GOOD.replace("name = \"org/service\"", "name = \"Org Service\""),
            GOOD.replace("ref = \"main\"", "ref = \"a b\""),
            format!("{GOOD}provider = \"svn\"\n"),
            format!("{GOOD}extra = 1\n"),
            GOOD.replace("url = \"https://example.org/org/service.git\"\n", ""),
        ] {
            assert!(check(&bad, "service", "s.toml").is_err(), "{bad}");
        }
    }
}
