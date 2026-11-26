//! Generated review units (#156): a checklist whose items are the pairs of
//! a scope gate's kept manifest, each covered by a worker's report prepared
//! `--for` it. Five members are ten units; eight covered read 8/10 and the
//! two owed are named; a report prepared for another unit answers nothing;
//! without a current pass of the scope there are no units and the checklist
//! is not covered; a sixth member makes fifteen units and every earlier row
//! stale; 65 members are refused with the count. Coverage is not
//! correctness: a report that says the wrong thing still covers its unit.
mod agent_support;
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use agent_support::{AGENT, outcome, setup};
use helpers::{RUN, put, set};
use support::{NOW, TestResult, World};
use writfold_core::checklist::Outcome;
use writfold_kb::Verb;
use writfold_kb::agents::AgentVerb;
use writfold_kb::agents::prepare::Prepare;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::gate::GateVerb;

const SCOPE: &str = "schema = \"writfold-gate/1\"\nid = \"related-scope\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"sh\", \"{gate}/scope.sh\", \"{artifacts}\"]\ntimeout_seconds = 30\nscope = [\"src/**\"]\nretain = [\"manifest.json\"]\n";
/// Writes a manifest of the members listed in `{gate}/members.txt`.
const SCRIPT: &str = "set -eu\nout=$1/manifest.json\nfirst=1\nprintf '{\"schema\": \"writfold-manifest/1\", \"coverage\": \"complete\", \"members\": [' > \"$out\"\nwhile read -r id; do\n  [ $first = 1 ] || printf ', ' >> \"$out\"\n  first=0\n  printf '{\"id\": \"%s\"}' \"$id\" >> \"$out\"\ndone < \"$(dirname \"$0\")/members.txt\"\nprintf ']}\\n' >> \"$out\"\n";
const UNITS: &str = "schema = \"writfold-gate/1\"\nid = \"pair-review\"\nversion = 1\nclass = \"checklist\"\nscope = [\"src/**\"]\nitems_from = \"related-scope/manifest.json\"\nunits = \"pairs\"\n";
const MODE: &str = "schema = \"writfold-delivery-mode/1\"\nmode = \"review\"\nchecks = [\"invocation.bound\"]\nagent = \"reviewer\"\n";
const PLAN: &str = "schema = \"writfold-plan/1\"\nname = \"claims-review\"\nsteps = [\n  \"review: deliver:review pair-review\",\n]\n";

fn members(w: &World, ids: &[&str]) -> TestResult {
    put(
        w,
        ".writfold/gates/members.txt",
        &format!("{}\n", ids.join("\n")),
    )
}

fn world() -> TestResult<World> {
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

fn gate(w: &World, verb: GateVerb) -> (String, u8) {
    let mut request = w.request(Verb::Gate(verb));
    request.now = Some(NOW + 100);
    outcome(&request)
}

fn scope(w: &World) -> TestResult {
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

fn status(w: &World) -> String {
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
fn report(w: &World, unit: &str, text: &str, n: i64) -> TestResult<String> {
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

fn cover(w: &World, unit: &str, candidate: &str) -> (String, u8) {
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

const PAIRS: [&str; 10] = [
    "a~b", "a~c", "a~d", "a~e", "b~c", "b~d", "b~e", "c~d", "c~e", "d~e",
];

#[test]
fn ten_units_eight_covered_then_all_then_a_sixth_member() -> TestResult {
    let w = world()?;
    let text = status(&w);
    assert!(
        text.contains("the scope gate related-scope is open"),
        "{text}"
    );
    scope(&w)?;
    let text = status(&w);
    assert!(text.contains("checklist 0/10 units covered"), "{text}");
    let mut cands = Vec::new();
    for (n, unit) in PAIRS.iter().enumerate() {
        // Coverage is not correctness: these reports say anything at all.
        let c = report(&w, unit, "The two records agree.\n", 10 + n as i64)?;
        cands.push(c);
    }
    // A report prepared for another unit answers nothing.
    let (text, exit) = cover(&w, "a~b", &cands[1]);
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("none of the cited references does"), "{text}");
    for (unit, c) in PAIRS.iter().zip(&cands).take(8) {
        let (text, exit) = cover(&w, unit, c);
        assert_eq!(exit, 0, "{unit}: {text}");
    }
    let text = status(&w);
    assert!(text.contains("checklist 8/10 units covered"), "{text}");
    assert!(
        text.contains("next: writfold kb gate cover pair-review c~e"),
        "{text}"
    );
    let (text, exit) = helpers::advance(&w)?;
    assert_ne!(exit, 0, "the step advanced with two units owed: {text}");
    for (unit, c) in PAIRS.iter().zip(&cands).skip(8) {
        assert_eq!(cover(&w, unit, c).1, 0);
    }
    let text = status(&w);
    assert!(text.contains("checklist 10/10 units covered"), "{text}");
    assert!(text.contains("gate pair-review [attested]"), "{text}");
    // A sixth member: fifteen units, every earlier row stale.
    members(&w, &["a", "b", "c", "d", "e", "f"])?;
    scope(&w)?;
    let text = status(&w);
    assert!(text.contains("checklist 0/15 units covered"), "{text}");
    assert!(text.contains("a~f"), "{text}");
    assert!(text.contains("a~b [stale]"), "{text}");
    Ok(())
}

#[test]
fn over_sixty_four_members_is_refused_with_the_count() -> TestResult {
    let w = world()?;
    let ids: Vec<String> = (0..65).map(|i| format!("r{i:02}")).collect();
    members(&w, &ids.iter().map(String::as_str).collect::<Vec<_>>())?;
    scope(&w)?;
    let text = status(&w);
    assert!(
        text.contains("the scope has 65 members; pair units take at most 64"),
        "{text}"
    );
    assert!(text.contains("gate pair-review [open]"), "{text}");
    Ok(())
}
