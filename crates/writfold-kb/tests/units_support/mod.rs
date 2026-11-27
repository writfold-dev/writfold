//! Shared setup for the generated-unit tests (#156, #165, #166): a scope
//! gate whose command writes a manifest from `.writfold/gates/members.txt`
//! (`id [digest [path]]` per line; `coverage.txt`, `schema.txt` and
//! `seed.txt` change what it says), a pair-unit checklist over it, and a
//! reviewer whose reports cover units.
#![allow(
    dead_code,
    reason = "shared by several test binaries; each one uses only part of it"
)]
use crate::agent_support::{self, AGENT, outcome, setup};
use crate::helpers::{self, RUN, put, set};
use crate::support::{NOW, TestResult, World};
use writfold_core::checklist::Outcome;
use writfold_kb::Verb;
use writfold_kb::agents::AgentVerb;
use writfold_kb::agents::prepare::Prepare;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::gate::GateVerb;

pub const SCOPE: &str = "schema = \"writfold-gate/1\"\nid = \"related-scope\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"sh\", \"{gate}/scope.sh\", \"{artifacts}\"]\ntimeout_seconds = 30\nscope = [\"src/**\"]\nretain = [\"manifest.json\"]\n";
/// Writes a manifest of the members listed in `{gate}/members.txt`.
pub const SCRIPT: &str = "set -eu\nd=$(dirname \"$0\")\nout=$1/manifest.json\ncov=complete; [ -f \"$d/coverage.txt\" ] && cov=$(cat \"$d/coverage.txt\")\nsch=writfold-manifest/1; [ -f \"$d/schema.txt\" ] && sch=$(cat \"$d/schema.txt\")\nprintf '{\"schema\": \"%s\", \"coverage\": \"%s\", ' \"$sch\" \"$cov\" > \"$out\"\nif [ -f \"$d/seed.txt\" ]; then printf '\"seed_path\": \"src/seed.rs\", \"seed_digest\": \"%s\", ' \"$(cat \"$d/seed.txt\")\" >> \"$out\"; fi\nprintf '\"members\": [' >> \"$out\"\nfirst=1\nwhile read -r id dg path; do\n  [ $first = 1 ] || printf ', ' >> \"$out\"\n  first=0\n  printf '{\"id\": \"%s\", \"path\": \"%s\", \"digest\": \"%s\"}' \"$id\" \"${path:-src/$id.rs}\" \"${dg:-sha256:$id}\" >> \"$out\"\ndone < \"$d/members.txt\"\nprintf ']}\\n' >> \"$out\"\n";
pub const UNITS: &str = "schema = \"writfold-gate/1\"\nid = \"pair-review\"\nversion = 1\nclass = \"checklist\"\nscope = [\"src/**\"]\nitems_from = \"related-scope/manifest.json\"\nunits = \"pairs\"\n";
pub const MODE: &str = "schema = \"writfold-delivery-mode/1\"\nmode = \"review\"\nchecks = [\"invocation.bound\"]\nagent = \"reviewer\"\n";
pub const PLAN: &str = "schema = \"writfold-plan/1\"\nname = \"claims-review\"\nsteps = [\n  \"review: deliver:review pair-review\",\n]\n";

pub fn members(w: &World, ids: &[&str]) -> TestResult {
    put(
        w,
        ".writfold/gates/members.txt",
        &format!("{}\n", ids.join("\n")),
    )
}

pub fn world() -> TestResult<World> {
    let w = setup()?;
    put(&w, ".writfold/gates/related-scope.toml", SCOPE)?;
    put(&w, ".writfold/gates/scope.sh", SCRIPT)?;
    put(&w, ".writfold/gates/pair-review.toml", UNITS)?;
    put(&w, ".writfold/deliveries/review.toml", MODE)?;
    put(&w, ".writfold/plans/claims-review.toml", PLAN)?;
    put(&w, ".writfold/agents/reviewer.toml", AGENT)?;
    members(&w, &["a", "b", "c", "d", "e"])?;
    set(&w, "claims-review")?;
    Ok(w)
}

pub fn gate(w: &World, verb: GateVerb) -> (String, u8) {
    let mut request = w.request(Verb::Gate(verb));
    request.now = Some(NOW + 100);
    outcome(&request)
}

pub fn scope(w: &World) -> TestResult {
    let (text, exit) = gate(
        w,
        GateVerb::Run {
            id: "related-scope".into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    );
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

pub fn status(w: &World) -> String {
    gate(
        w,
        GateVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )
    .0
}

/// Prepare `reviewer` for `unit`, deliver `text` as its report and check
/// it; returns the candidate.
pub fn report(w: &World, unit: &str, text: &str, n: i64) -> TestResult<String> {
    let before = agent_support::invocations(w);
    let verb = Verb::Agent(AgentVerb::Prepare(Prepare {
        id: "reviewer".into(),
        input: "task.md".into(),
        out: format!("pkg-{n}").into(),
        retry_of: None,
        by: None,
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
        for_item: Some(format!("pair-review.{unit}")),
    }));
    let (text_, exit) = outcome(&w.request(verb));
    assert_eq!(exit, 0, "{text_}");
    let id = (agent_support::invocations(w).into_iter())
        .find(|i| !before.contains(i))
        .ok_or("no invocation")?
        .trim_end_matches(".json")
        .to_owned();
    let from = w.base.join(format!("report-{n}.md"));
    std::fs::write(&from, text)?;
    let cands = helpers::ids(w)?;
    let p = writfold_kb::deliver::candidate::Prepare {
        mode: "review".into(),
        from,
        audience: None,
        destination: None,
        run: Some(RUN.into()),
        invocation: Some(id),
    };
    assert_eq!(helpers::deliver(w, DeliverVerb::Prepare(p), NOW + n)?.1, 0);
    let c = (helpers::ids(w)?.into_iter())
        .find(|c| !cands.contains(c))
        .ok_or("no candidate")?;
    let (text, _) = helpers::check(w, &c, NOW + n)?;
    assert!(text.contains("invocation.bound [satisfied]"), "{text}");
    Ok(c)
}

pub fn cover(w: &World, unit: &str, candidate: &str) -> (String, u8) {
    gate(
        w,
        GateVerb::Cover(writfold_kb::gate::checklist::Cover {
            gate: "pair-review".into(),
            item: unit.into(),
            outcome: Outcome::Supported,
            references: vec![format!("delivery:{candidate}")],
            reason: None,
            note: None,
            by: "agent".into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        }),
    )
}
