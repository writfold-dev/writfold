use super::executable::descriptor_path;
use super::{DoltError, DoltExecutable, DoltQuery};
use crate::fs::{
    ContainedRoot, FileIdentity, FsError, LockKey, LockedFile, ManagedLocks, PinnedFile,
    RelativePath,
};
use crate::process::{self, ProcessLimits};
use rustix::fs::{AtFlags, fstat, statat};
use rustix::process::geteuid;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use writfold_domain::dolt::DoltReadPin;
use writfold_domain::{ByteCount, RowLimit};

#[derive(Debug)]
pub struct DoltDatabase {
    root: ContainedRoot,
    home: ContainedRoot,
    metadata: ContainedRoot,
    locks: ManagedLocks,
    lock_target: PinnedFile,
    lock_key: LockKey,
    lock_identity: FileIdentity,
    metadata_limit: ByteCount,
}

impl DoltDatabase {
    /// Admit only already-provisioned private managed directories and lock file.
    /// No init, migration, checkout, directory creation, or lock creation occurs.
    pub fn open(
        root: ContainedRoot,
        home: ContainedRoot,
        metadata_limit: ByteCount,
    ) -> Result<Self, DoltError> {
        check_private(&root)?;
        check_private(&home)?;
        let root_stat = fstat(root.descriptor()).map_err(FsError::from)?;
        let home_stat = fstat(home.descriptor()).map_err(FsError::from)?;
        if root_stat.st_dev == home_stat.st_dev && root_stat.st_ino == home_stat.st_ino {
            return Err(DoltError::UnsupportedProfile(
                "database and HOME must be separate",
            ));
        }
        let metadata = root.open_child(&relative(".dolt")?)?;
        let coordination = root.open_child(&relative("coordination")?)?;
        check_private(&coordination)?;
        let lock_target = coordination.pin(&relative("database.lock")?)?;
        let lock = lock_target.open_read()?;
        if lock.size() > metadata_limit.get() {
            return Err(DoltError::Budget {
                stream: "lock metadata",
                limit: metadata_limit.get(),
            });
        }
        let lock_identity = lock.identity();
        drop(lock);
        let database = Self {
            root,
            home,
            metadata,
            locks: ManagedLocks::from_root(coordination),
            lock_target,
            lock_key: LockKey::new("database")?,
            lock_identity,
            metadata_limit,
        };
        database.check_current()?;
        Ok(database)
    }

    pub fn acquire(&self) -> Result<DoltLease<'_>, DoltError> {
        self.check_current()?;
        let lock = self
            .lock_target
            .try_lock_existing(&self.locks, &self.lock_key)?;
        if lock.lock_identity() != self.lock_identity {
            return Err(FsError::ChangedSource.into());
        }
        let lease = DoltLease {
            database: self,
            lock,
        };
        lease.check_current()?;
        Ok(lease)
    }

    fn check_current(&self) -> Result<(), DoltError> {
        check_private(&self.root)?;
        check_private(&self.home)?;
        self.metadata.check_current()?;
        self.locks.check_current()?;
        for root in [&self.root, &self.metadata, &self.home] {
            for name in ["sql-server.info", "sql-server.lock"] {
                refuse_entry(root, name)?;
            }
        }
        // Local repository configuration is administratively managed and bounded.
        // HOME never supplies global settings, server profiles or credentials.
        check_config(&self.metadata, self.metadata_limit)?;
        check_home(&self.home, self.metadata_limit)?;
        Ok(())
    }
}

/// The coordination lock outlives every command, including all group cleanup.
#[derive(Debug)]
pub struct DoltLease<'a> {
    database: &'a DoltDatabase,
    lock: LockedFile<'a>,
}

impl DoltLease<'_> {
    pub fn check_current(&self) -> Result<(), DoltError> {
        self.lock.check_current()?;
        self.database.check_current()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn query(
        &self,
        executable: &DoltExecutable<'_>,
        pin: &DoltReadPin,
        query: DoltQuery,
        rows: RowLimit,
        stdout: ByteCount,
        stderr: ByteCount,
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Result<Vec<u8>, DoltError> {
        process::check_stop(deadline, cancelled)?;
        self.check_current()?;
        let sql = query.render(pin.commit(), rows)?;
        let home = descriptor_path(self.database.home.descriptor());
        let mut command = Command::new(executable.path()?);
        command
            .env_clear()
            .current_dir(descriptor_path(self.database.root.descriptor()))
            .env("HOME", &home)
            .env("DOLT_ROOT_PATH", &home)
            .env("DOLT_DISABLE_EVENT_FLUSH", "1")
            .env("DOLT_DISABLE_VERSION_CHECK", "1")
            .env("GOMAXPROCS", "2")
            .env("TZ", "UTC")
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .args([
                "sql",
                "--disable-auto-gc",
                "--result-format",
                "json",
                "--query",
            ])
            .arg(sql);
        let result = process::run(
            &mut command,
            &[],
            ProcessLimits::new(stdout, stderr),
            deadline,
            cancelled,
            executable.supervisor(),
        );
        // run returns only after group cleanup, including failure paths. Check
        // retained filesystem identities before exposing any successful bytes.
        self.check_current()?;
        executable.path()?;
        let output = result?;
        process::check_stop(deadline, cancelled)?;
        Ok(output.stdout)
    }
}

fn relative(path: &str) -> Result<RelativePath, FsError> {
    RelativePath::new(path).map_err(FsError::from)
}

fn check_private(root: &ContainedRoot) -> Result<(), DoltError> {
    root.check_current()?;
    let stat = fstat(root.descriptor()).map_err(FsError::from)?;
    if stat.st_uid != geteuid().as_raw() || stat.st_mode & 0o077 != 0 {
        return Err(DoltError::UnsupportedProfile(
            "owner-only managed directory required",
        ));
    }
    Ok(())
}

fn refuse_entry(root: &ContainedRoot, name: &str) -> Result<(), DoltError> {
    match statat(root.descriptor(), name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(rustix::io::Errno::NOENT) => Ok(()),
        Err(error) => Err(FsError::from(error).into()),
        Ok(_) => Err(DoltError::UnsupportedProfile(
            "native server discovery or global configuration/credentials",
        )),
    }
}

fn check_config(root: &ContainedRoot, limit: ByteCount) -> Result<(), DoltError> {
    let file = root.pin(&relative("config.json")?)?;
    match file.open_read() {
        Err(FsError::Missing) => Ok(()),
        Err(error) => Err(error.into()),
        Ok(opened) => {
            let stat = fstat(opened.descriptor()).map_err(FsError::from)?;
            if stat.st_size < 0 || stat.st_size as u64 > limit.get() {
                return Err(DoltError::Budget {
                    stream: "configuration",
                    limit: limit.get(),
                });
            }
            opened.check_unchanged()?;
            Ok(())
        }
    }
}

fn check_home(home: &ContainedRoot, limit: ByteCount) -> Result<(), DoltError> {
    for name in ["config_global.json", "config.json", "creds"] {
        refuse_entry(home, name)?;
    }
    match home.open_child(&relative(".dolt")?) {
        Err(FsError::Missing) => Ok(()),
        Err(error) => Err(error.into()),
        Ok(global) => {
            for name in ["config.json", "creds", "sql-server.info", "sql-server.lock"] {
                refuse_entry(&global, name)?;
            }
            super::config::empty_global(&global, limit)?;
            global.check_current()?;
            Ok(())
        }
    }
}
