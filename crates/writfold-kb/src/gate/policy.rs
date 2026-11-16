//! A gate policy as the verbs read it: whether any policy is configured.

/// The line `resume`, the session-start context and `kb policy` show when
/// nothing is configured.
pub const NONE_CONFIGURED: &str = "policy: none configured (continuity only: checkpoints, records and citations work without one)";

/// The directories under `.writfold/` that hold policy files.
const POLICY_DIRS: [&str; 4] = ["gates", "plans", "guards", "deliveries"];

/// Whether `top/.writfold/` holds any gate, plan template, required file,
/// guard rule or delivery file (a `.toml` in one of those directories).
fn holds(top: &std::path::Path) -> bool {
    POLICY_DIRS.iter().any(|dir| {
        std::fs::read_dir(top.join(".writfold").join(dir)).is_ok_and(|entries| {
            entries.flatten().any(|e| {
                let path = e.path();
                path.extension().is_some_and(|x| x == "toml")
            })
        })
    })
}

/// Whether the KB root or the subject repository configures any policy.
pub fn configured(root: &std::path::Path, repo: Option<&std::path::Path>) -> bool {
    holds(root) || repo.is_some_and(holds)
}
