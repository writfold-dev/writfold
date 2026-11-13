//! `cite make` (a code citation, or with `--kind` a checked kind's token
//! from its checker) and `checker list`. Parsing only.
use super::args::Common;
use super::writes::{get, switch};
use writfold_kb::Verb;
use writfold_kb::stale::checker::verb::CheckerVerb;
use writfold_kb::write::Write;
use writfold_kb::write::cite::CiteMake;

const USAGE: &str = "usage: cite make <repo> <path> <a-b> | cite make --kind K key=value …";

/// `cite …` or `checker …`.
pub fn verb(name: &str, common: &Common, positional: &[String]) -> Result<Verb, String> {
    match (name, common.kinds.as_slice(), positional) {
        ("checker", [], [list]) if list == "list" => Ok(Verb::Checker(CheckerVerb::List)),
        ("checker", _, _) => Err("usage: checker list".into()),
        ("cite", [], [make, repo, path, range]) if make == "make" => {
            code(common, repo, path, range).map(Verb::Write)
        }
        ("cite", [kind], [make, pairs @ ..]) if make == "make" => checked(common, kind, pairs),
        _ => Err(USAGE.into()),
    }
}

fn code(common: &Common, repo: &str, path: &str, range: &str) -> Result<Write, String> {
    let (a, b) = range.split_once('-').unwrap_or((range, range));
    let (Ok(first), Ok(last)) = (a.parse::<u32>(), b.parse::<u32>()) else {
        return Err(format!("{range:?} is not a line range a-b"));
    };
    Ok(Write::CiteMake(CiteMake {
        repo: repo.to_owned(),
        path: path.to_owned(),
        first,
        last,
        allow_dirty: switch(common, "allow-dirty"),
        sym: get(common, "sym"),
        ttl: get(common, "ttl"),
    }))
}

fn checked(common: &Common, kind: &str, pairs: &[String]) -> Result<Verb, String> {
    if let Some(flag) = common.values.keys().chain(common.switches.iter()).next() {
        return Err(format!("--{flag} does not apply to cite make --kind"));
    }
    let mut fields: Vec<(String, String)> = Vec::new();
    for pair in pairs {
        let Some((key, value)) = pair.split_once('=').filter(|(k, _)| !k.is_empty()) else {
            return Err(format!("{pair:?} is not key=value"));
        };
        if fields.iter().any(|(k, _)| k == key) {
            return Err(format!("{key}= given twice"));
        }
        fields.push((key.to_owned(), value.to_owned()));
    }
    Ok(Verb::Checker(CheckerVerb::Make {
        kind: kind.to_owned(),
        fields,
    }))
}

#[cfg(test)]
mod tests {
    use super::super::args::parse;
    use std::path::PathBuf;
    use writfold_kb::Verb;
    use writfold_kb::stale::checker::verb::CheckerVerb;

    fn verb(args: &str) -> Result<Verb, String> {
        let args: Vec<String> = args.split(' ').map(str::to_owned).collect();
        Ok(parse(&args, PathBuf::from("/"))?.ok_or("a request")?.verb)
    }

    #[test]
    fn checked_make_and_checker_list() -> Result<(), String> {
        let made = verb("cite make --kind src depot=main path=a/b.c L=3-9")?;
        assert!(matches!(
            made,
            Verb::Checker(CheckerVerb::Make { ref kind, ref fields })
                if kind == "src" && fields.len() == 3 && fields[2] == ("L".into(), "3-9".into())
        ));
        assert!(verb("cite make --kind src depot=a depot=b").is_err());
        assert!(verb("cite make --kind src novalue").is_err());
        assert!(verb("cite make --kind src,web a=b").is_err());
        assert!(verb("cite make --kind src a=b --ttl 3d").is_err());
        assert!(verb("cite make demo src/a.rs 3-5").is_ok());
        assert!(matches!(
            verb("checker list")?,
            Verb::Checker(CheckerVerb::List)
        ));
        assert!(verb("checker run").is_err());
        Ok(())
    }
}
