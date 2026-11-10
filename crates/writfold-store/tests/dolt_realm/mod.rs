//! Disposable Dolt realm for the storage conformance suite. Fixture provisioning
//! uses the native CLI as the fixture owner; every product read and write goes
//! through the Rust boundary. Helpers return errors and never assert.
pub mod approval;

use std::fs;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use writfold_domain::SchemaEpoch;
use writfold_domain::dolt::{DOLT_READ_PROFILE, DoltReadPin};
use writfold_runtime::dolt::query_contract_digest;
use writfold_store::backend::conformance::{AFTER, BEFORE, ConformanceFixture, FixtureRecord};
use writfold_store::dolt::{SCHEMA_SQL, schema_digest};
use writfold_wire::dolt_operation::DoltTarget;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);
// Realm and KB equal the public synthetic Dolt approval fixtures' policy, so
// the writer test can sign real approvals for this disposable realm.
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

/// Admitted executable path and SHA-256 from the integration profile.
pub fn admitted_candidate() -> Result<(PathBuf, String)> {
    let profile = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../qualification/dolt-integration-profile.json");
    let profile: Profile = serde_json::from_slice(&fs::read(profile)?)?;
    Ok((profile.candidate.executable, profile.candidate.sha256))
}

pub struct Realm {
    native: Native,
    fixture: ConformanceFixture<DoltReadPin>,
    head: String,
}

impl Realm {
    pub fn provision(dolt: &Path, producer: &str) -> Result<Self> {
        let native = Native {
            root: private_root()?,
            dolt: dolt.to_owned(),
        };
        for name in ["database", "reader-home", "writer-home"] {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(native.path(name))?;
        }
        let author = ["WritfoldFixture", "fixture@example.invalid"];
        native.run(&["init", "--name", author[0], "--email", author[1]])?;
        for (key, value) in [("user.name", author[0]), ("user.email", author[1])] {
            native.run(&["config", "--local", "--add", key, value])?;
        }
        native.sql(SCHEMA_SQL)?;
        native.sql(&metadata(producer)?)?;
        let before = native.commit(BEFORE, "before")?;
        let after = native.commit(AFTER, "after")?;
        fs::DirBuilder::new()
            .mode(0o700)
            .create(native.path("database/coordination"))?;
        for lock in ["database.lock", "writer.lock"] {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(native.path(&format!("database/coordination/{lock}")))?;
        }
        let fixture = ConformanceFixture {
            before: pin(&before, producer)?,
            after: pin(&after, producer)?,
            missing: pin(&"0".repeat(32), producer)?,
        };
        let head = native.head()?;
        Ok(Self {
            native,
            fixture,
            head,
        })
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.native.path(name)
    }

    pub fn fixture(&self) -> &ConformanceFixture<DoltReadPin> {
        &self.fixture
    }

    pub fn head_at_provision(&self) -> &str {
        &self.head
    }

    pub fn head(&self) -> Result<String> {
        self.native.head()
    }

    /// Native `dolt status` reports a clean working set.
    pub fn clean(&self) -> Result<bool> {
        let out = self.native.run(&["status"])?;
        Ok(String::from_utf8(out)?.contains("nothing to commit, working tree clean"))
    }

    /// Native branch names (retained roots included).
    pub fn branches(&self) -> Result<String> {
        Ok(String::from_utf8(self.native.run(&["branch"])?)?)
    }

    /// The realm identity a writer owns.
    pub fn target(&self) -> DoltTarget {
        let pin = &self.fixture.after;
        DoltTarget::new(
            pin.realm().clone(),
            pin.kb().clone(),
            pin.database().clone(),
            pin.epoch().clone(),
            pin.profile().clone(),
            pin.producer().clone(),
        )
    }

    /// The `AFTER` commit named in a realm this writer does not own.
    pub fn foreign(&self) -> Result<DoltReadPin> {
        let pin = &self.fixture.after;
        Ok(DoltReadPin::new(
            "other-realm".parse()?,
            pin.kb().clone(),
            pin.database().clone(),
            pin.commit().clone(),
            pin.epoch().clone(),
            pin.profile().clone(),
            pin.producer().clone(),
        ))
    }
}

/// Native CLI fixture owner for one private out-of-tree directory.
struct Native {
    root: PathBuf,
    dolt: PathBuf,
}

impl Native {
    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn head(&self) -> Result<String> {
        let out = self.run(&["sql", "-r", "csv", "-q", "SELECT HASHOF('HEAD') AS h"])?;
        let text = String::from_utf8(out)?;
        let mut lines = text.lines();
        if lines.next() != Some("h") {
            return Err(format!("unexpected HEAD output: {text}").into());
        }
        Ok(lines.next().ok_or("missing HEAD row")?.trim().to_owned())
    }

    fn commit(&self, records: &[FixtureRecord], name: &str) -> Result<String> {
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
        let message = format!("B0 conformance {name}");
        self.run(&["commit", "--date", "2026-11-09T00:00:00Z", "-m", &message])?;
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
            .env("LANG", "C.UTF-8")
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

impl Drop for Native {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn private_root() -> Result<PathBuf> {
    for _ in 0..128 {
        let root = std::env::temp_dir().join(format!(
            ".writfold-backend-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::DirBuilder::new().mode(0o700).create(&root) {
            Ok(()) => return Ok(root),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err("disposable realm names exhausted".into())
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
