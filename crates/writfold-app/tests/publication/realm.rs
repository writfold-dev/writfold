//! A disposable publication realm: one Dolt database (provisioned through the
//! native CLI as fixture owner at the conformance `AFTER` state) and one new
//! bare SHA-256 Git release repository. Helpers return errors; never assert.
use super::world::Result;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use writfold_domain::SchemaEpoch;
use writfold_domain::dolt::{DOLT_READ_PROFILE, DoltReadPin};
use writfold_runtime::dolt::query_contract_digest;
use writfold_store::backend::conformance::{AFTER, BEFORE, FixtureRecord};
use writfold_store::dolt::{SCHEMA_SQL, schema_digest};
use writfold_wire::dolt_operation::DoltTarget;

static NEXT: AtomicU64 = AtomicU64::new(0);
// Equal to the public synthetic approval policy's realm and KB.
const IDENTITY: [(&str, &str); 4] = [
    ("realm", "test-realm"),
    ("kb", "test-kb"),
    ("database_id", "corpus-backend"),
    ("epoch", "epoch-1"),
];

#[derive(serde::Deserialize)]
struct Profile {
    candidate: Candidate,
}

#[derive(serde::Deserialize)]
struct Candidate {
    executable: PathBuf,
    sha256: String,
}

/// Admitted Dolt executable path and SHA-256 from the integration profile.
pub fn admitted_dolt() -> Result<(PathBuf, String)> {
    let profile = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../qualification/dolt-integration-profile.json");
    let profile: Profile = serde_json::from_slice(&fs::read(profile)?)?;
    Ok((profile.candidate.executable, profile.candidate.sha256))
}

pub struct Realm {
    root: PathBuf,
    dolt: PathBuf,
    pub before: DoltReadPin,
    pub after: DoltReadPin,
}

impl Realm {
    pub fn provision(dolt: &Path, producer: &str) -> Result<Self> {
        let root = private_root()?;
        let mut realm = Self {
            before: pin(&"0".repeat(32), producer)?,
            after: pin(&"0".repeat(32), producer)?,
            root,
            dolt: dolt.to_owned(),
        };
        for name in ["database", "reader-home", "writer-home"] {
            fs::DirBuilder::new().mode(0o700).create(realm.path(name))?;
        }
        let author = ["WritfoldFixture", "fixture@example.invalid"];
        realm.run(&["init", "--name", author[0], "--email", author[1]])?;
        for (key, value) in [("user.name", author[0]), ("user.email", author[1])] {
            realm.run(&["config", "--local", "--add", key, value])?;
        }
        realm.sql(SCHEMA_SQL)?;
        realm.sql(&metadata(producer)?)?;
        realm.before = pin(&realm.commit(BEFORE)?, producer)?;
        realm.after = pin(&realm.commit(AFTER)?, producer)?;
        let coordination = realm.path("database/coordination");
        fs::DirBuilder::new().mode(0o700).create(&coordination)?;
        for lock in ["database.lock", "writer.lock"] {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(coordination.join(lock))?;
        }
        Ok(realm)
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// The realm identity the writer owns and approvals target.
    pub fn target(&self) -> DoltTarget {
        let pin = &self.after;
        DoltTarget::new(
            pin.realm().clone(),
            pin.kb().clone(),
            pin.database().clone(),
            pin.epoch().clone(),
            pin.profile().clone(),
            pin.producer().clone(),
        )
    }

    /// Native `dolt branch` output (retained roots included).
    pub fn branches(&self) -> Result<String> {
        Ok(String::from_utf8(self.run(&["branch"])?)?)
    }

    /// Native count of commits whose message contains `text`.
    pub fn commits_mentioning(&self, text: &str) -> Result<usize> {
        let log = String::from_utf8(self.run(&["log", "--oneline"])?)?;
        Ok(log.lines().filter(|line| line.contains(text)).count())
    }

    /// The native head commit, read by the fixture owner.
    pub fn head(&self) -> Result<String> {
        let out = self.run(&["sql", "-r", "csv", "-q", "SELECT HASHOF('HEAD') AS h"])?;
        let text = String::from_utf8(out)?;
        Ok(text
            .lines()
            .nth(1)
            .ok_or("missing HEAD row")?
            .trim()
            .to_owned())
    }

    fn commit(&self, records: &[FixtureRecord]) -> Result<String> {
        let mut statements = vec![
            "DELETE FROM wf_edges".to_owned(),
            "DELETE FROM wf_nodes".to_owned(),
        ];
        for record in records {
            statements.push(format!(
                "INSERT INTO wf_nodes (id,kind,label,home_path) VALUES ({},{},{},NULL)",
                literal(record.key),
                literal(record.kind),
                literal(record.content)
            ));
        }
        self.sql(&statements.join(";"))?;
        self.run(&["add", "-A"])?;
        self.run(&["commit", "--date", "2026-11-10T00:00:00Z", "-m", "fixture"])?;
        let commit = self.head()?;
        self.run(&["branch", &format!("wf-retained/{commit}"), &commit])?;
        Ok(commit)
    }

    fn sql(&self, sql: &str) -> Result {
        self.run(&["sql", "--disable-auto-gc", "-q", sql]).map(drop)
    }

    fn run(&self, args: &[&str]) -> Result<Vec<u8>> {
        let home = self.path("writer-home");
        let output = Command::new(&self.dolt)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &home)
            .env("DOLT_ROOT_PATH", &home)
            .env("DOLT_DISABLE_EVENT_FLUSH", "1")
            .env("DOLT_DISABLE_VERSION_CHECK", "1")
            .env("TZ", "UTC")
            .env("LC_ALL", "C.UTF-8")
            .current_dir(self.path("database"))
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("dolt {} failed: {stderr}", args.join(" ")).into());
        }
        Ok(output.stdout)
    }
}

impl Drop for Realm {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn private_root() -> Result<PathBuf> {
    let root = std::env::temp_dir().join(format!(
        ".writfold-publication-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    Ok(root)
}

fn literal(value: &str) -> String {
    let hex: String = value.bytes().map(|byte| format!("{byte:02x}")).collect();
    format!("CONVERT(X'{hex}' USING utf8mb4)")
}

fn metadata(producer: &str) -> Result<String> {
    let mut columns = vec!["id"];
    let mut values = vec!["1".to_owned()];
    for (column, value) in IDENTITY {
        columns.push(column);
        values.push(literal(value));
    }
    for (column, value) in [
        ("schema_digest", schema_digest()?.as_str()),
        ("query_digest", query_contract_digest()?.as_str()),
        ("storage_profile", DOLT_READ_PROFILE),
        ("producer", producer),
    ] {
        columns.push(column);
        values.push(literal(value));
    }
    Ok(format!(
        "INSERT INTO wf_reader_meta ({}) VALUES ({})",
        columns.join(","),
        values.join(",")
    ))
}

fn pin(commit: &str, producer: &str) -> Result<DoltReadPin> {
    let [realm, kb, database, epoch] = IDENTITY.map(|(_, value)| value);
    Ok(DoltReadPin::new(
        realm.parse()?,
        kb.parse()?,
        database.parse()?,
        commit.parse()?,
        SchemaEpoch::new(
            epoch.parse()?,
            schema_digest()?.clone(),
            query_contract_digest()?.clone(),
        ),
        DOLT_READ_PROFILE.parse()?,
        producer.parse()?,
    ))
}
