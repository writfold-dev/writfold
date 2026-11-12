//! `writfold kb edit apply|recover`: arguments only; the adapter is
//! `writfold_kb::edit`.
use super::args::Common;
use super::writes::{get, switch};
use std::path::PathBuf;
use writfold_kb::Verb;
use writfold_kb::edit::EditVerb;

pub const USAGE: &str = "usage: edit apply RECIPE.spl [RECIPE.spl ...] [--run ID] [--worktree DIR] [--dry-run] | edit recover [--run ID] [--abandon]";

/// The edit verb, or why the arguments do not name one.
pub fn verb(common: &Common, positional: &[String]) -> Result<Verb, String> {
    let run = get(common, "run");
    let verb = match positional {
        [sub, recipes @ ..] if sub == "apply" && !recipes.is_empty() => {
            if switch(common, "abandon") {
                return Err("--abandon belongs to edit recover".into());
            }
            EditVerb::Apply {
                recipes: recipes.iter().map(PathBuf::from).collect(),
                run,
                worktree: get(common, "worktree").map(PathBuf::from),
                dry_run: switch(common, "dry-run"),
            }
        }
        [sub] if sub == "recover" => {
            if get(common, "worktree").is_some() || switch(common, "dry-run") {
                return Err("edit recover takes only --run and --abandon".into());
            }
            EditVerb::Recover {
                run,
                abandon: switch(common, "abandon"),
            }
        }
        _ => return Err(USAGE.into()),
    };
    Ok(Verb::Edit(verb))
}

#[cfg(test)]
mod tests {
    use super::super::args::parse;
    use std::path::PathBuf;
    use writfold_kb::Verb;
    use writfold_kb::edit::EditVerb;

    fn verb(args: &str) -> Result<Verb, String> {
        let args: Vec<String> = args.split(' ').map(str::to_owned).collect();
        Ok(parse(&args, PathBuf::from("/"))?.ok_or("a request")?.verb)
    }

    #[test]
    fn edit_verbs() -> Result<(), String> {
        let apply = verb("edit apply a.spl b.spl --run r --worktree w --dry-run")?;
        assert!(matches!(
            apply,
            Verb::Edit(EditVerb::Apply { ref recipes, run: Some(_), worktree: Some(_), dry_run: true })
                if recipes.len() == 2
        ));
        assert!(matches!(
            verb("edit recover --abandon")?,
            Verb::Edit(EditVerb::Recover {
                run: None,
                abandon: true
            })
        ));
        for bad in [
            "edit",
            "edit apply",
            "edit apply a.spl --abandon",
            "edit recover --dry-run",
            "edit recover x",
            "edit apply a.spl --note n",
            "edit fold",
        ] {
            assert!(verb(bad).is_err(), "{bad}");
        }
        Ok(())
    }
}
