use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use writfold_domain::{ByteCount, ContentDigest, GitObjectFormat, GitObjectId};
use writfold_runtime::fs::{ContainedRoot, PinnedFile, RelativePath};
use writfold_runtime::git::{GitBudgets, GitExecutable, GitRepository};
pub use writfold_runtime::process::{ProcessError, ProcessSupervisor};

pub type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);
pub static ACTIVE: AtomicBool = AtomicBool::new(false);
static SUPERVISOR: LazyLock<std::result::Result<ProcessSupervisor, ProcessError>> =
    LazyLock::new(ProcessSupervisor::enable);

pub struct Fixture {
    pub path: PathBuf,
}

impl Fixture {
    pub fn new(format: GitObjectFormat) -> Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "writfold-git-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        let fixture = Self { path };
        let format = match format {
            GitObjectFormat::Sha1 => "sha1",
            GitObjectFormat::Sha256 => "sha256",
        };
        fixture.git(
            &["init", "--template=", &format!("--object-format={format}")],
            &[],
        )?;
        Ok(fixture)
    }

    pub fn git(&self, args: &[&str], input: &[u8]) -> Result<Vec<u8>> {
        self.git_os(&args.iter().map(OsStr::new).collect::<Vec<_>>(), input)
    }

    pub fn git_os(&self, args: &[&OsStr], input: &[u8]) -> Result<Vec<u8>> {
        let mut child = Command::new("/usr/bin/git")
            .env_clear()
            .env("PATH", "")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .current_dir(&self.path)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        child
            .stdin
            .take()
            .ok_or("missing stdin")?
            .write_all(input)?;
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(format!("fixture Git failed: {:?}", output.stderr).into());
        }
        Ok(output.stdout)
    }

    pub fn commit(&self) -> Result<GitObjectId> {
        self.git(&["add", "--all"], &[])?;
        self.git(&["commit", "--quiet", "-m", "fixture"], &[])?;
        oid(&self.git(&["rev-parse", "HEAD"], &[])?)
    }

    pub fn blob(&self, bytes: &[u8]) -> Result<GitObjectId> {
        oid(&self.git(&["hash-object", "-w", "--stdin", "--no-filters"], bytes)?)
    }

    pub fn repository(&self, format: GitObjectFormat) -> Result<GitRepository> {
        Ok(GitRepository::admit(
            ContainedRoot::open(self.path.join(".git"))?,
            format,
            count(65536)?,
        )?)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn count(value: u64) -> Result<ByteCount> {
    Ok(ByteCount::new(value)?)
}
pub fn budgets() -> Result<GitBudgets> {
    Ok(GitBudgets::new(
        count(1 << 20)?,
        count(1 << 16)?,
        count(1 << 20)?,
        Duration::from_secs(20),
    ))
}
pub fn oid(bytes: &[u8]) -> Result<GitObjectId> {
    Ok(std::str::from_utf8(bytes.strip_suffix(b"\n").ok_or("no oid newline")?)?.parse()?)
}
pub fn executable_file() -> Result<PinnedFile> {
    Ok(ContainedRoot::open("/usr/bin")?.pin(&RelativePath::new("git")?)?)
}
pub fn executable(file: &PinnedFile) -> Result<GitExecutable<'_>> {
    // Exact maintained qualification profile, not observed/self-approved bytes.
    let digest: ContentDigest =
        "eb54b535467e0975c1284d4c4d08eea779aeb248a5d34b02e2ee171382e22201".parse()?;
    Ok(GitExecutable::admit(
        file,
        &digest,
        count(16 << 20)?,
        supervisor()?,
    )?)
}

pub fn supervisor() -> Result<&'static ProcessSupervisor> {
    match &*SUPERVISOR {
        Ok(supervisor) => Ok(supervisor),
        Err(error) => Err(format!("process supervision admission failed: {error}").into()),
    }
}
