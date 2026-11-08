# Native contained filesystem effects

`writfold-runtime::fs` implements the descriptor-relative filesystem portion of P05 and plan/04. It uses `rustix = 1.1.4` filesystem operations, checked domain `ContentDigest` values, SHA-256 and typed errors. The crate forbids unsafe Rust. This is not completion of P05, a knowledge-release transaction, a Git/Dolt adapter, or a claim of lock-free filesystem correctness.

## Public API

Capability constructors retain private fields. Plain stage witnesses expose fields for lossless owner-side persistence; constructing one does not create a capability. Filesystem paths are distinct from public UTF-8 domain identifiers.

| Type | Public operations |
| --- | --- |
| `RelativePath` | `new(impl AsRef<Path>) -> Result<Self, PathError>`; `as_path() -> &Path` |
| `ContainedRoot` | `open(impl AsRef<Path>) -> Result<Self, FsError>`; `create(impl AsRef<Path>) -> Result<Self, FsError>`; `pin(&RelativePath) -> Result<PinnedFile, FsError>` |
| `PinnedFile` | `open_read() -> Result<OpenedFile<'_>, FsError>`; `read(ByteCount) -> Result<ReadResult, FsError>`; `try_lock(&ManagedLocks, &LockKey) -> Result<LockedFile<'_>, FsError>` |
| `OpenedFile` | `read(ByteCount) -> Result<ReadResult, FsError>` |
| `ManagedLocks` | `open(impl AsRef<Path>) -> Result<Self, FsError>`; `create(impl AsRef<Path>) -> Result<Self, FsError>` |
| `LockKey` | `new(&str) -> Result<Self, FsError>`; `as_str() -> &str` |
| `LockedFile` | `read(ByteCount) -> Result<ReadResult, FsError>`; `lock_identity() -> FileIdentity`; `create(&mut self, &[u8], ByteCount) -> Result<SyncedWrite, WriteError>`; `compare_and_write(&mut self, Option<&ContentDigest>, &[u8], ByteCount, ByteCount) -> Result<SyncedWrite, WriteError>` |
| `ReadResult` | `bytes() -> &[u8]`; `into_bytes() -> Vec<u8>`; `digest() -> &ContentDigest`; `identity() -> FileIdentity`; `extent() -> ReadExtent` |
| `SyncedWrite` | `digest() -> &ContentDigest`; `bytes() -> usize`; `identity() -> FileIdentity` |
| `FileIdentity` | `device() -> u64`; `inode() -> u64` |
| `ReplacementObservation` | `preimage() -> &StageImage`; obtained only through `LockedFile::observe_replacement(ByteCount)` |
| `StagedReplacement` | `witness() -> &StageWitness`; consuming `install() -> Result<InstalledStage, InstallError>` |
| `StageWitness` | Exact parent/lock/stage identities, target name, preimage/candidate bytes, digests and full admitted POSIX metadata; untrusted when rehydrated |

The two `compare_and_write` limits are **current-source read limit**, then **replacement write limit**. `None` expects an absent destination. `Some(digest)` requires a complete current read with exactly that content digest. All mutation methods borrow the guard mutably, preventing simultaneous mutations through one shared flock guard. `PinnedFile::try_lock` borrows both the pinned target and namespace for the guard's lifetime; it does not retain a borrow of the key.

`ReadExtent` is `Complete` or `Truncated { limit: ByteCount, observed_size: u64 }`. A truncated result's digest describes **only its returned prefix**. It never authenticates the unread suffix. Returned bytes are exact, with no decoding, newline conversion, or normalization. Digest strings use `ContentDigest::try_from`, not an unchecked public string factory.

`PathError` distinguishes absolute paths, empty components, dot/parent components and NUL. `FsError` distinguishes invalid paths/lock keys, missing entries, permission denial, symlinks, nonregular files, nondirectories, changed sources, hardlink refusal, busy locks, existing destinations, stale expected revisions, budgets, unsupported operations and other IO. Budget kinds are read, write, allocation and temporary-name attempts. Underlying errors remain sources where applicable. Generated-digest validation failure is explicit. Root initialization that fails synchronization reports `RootDurabilityUnknown` rather than a successful root handle.

## Admission and retained descriptors

`ContainedRoot::open` never creates anything. Absolute roots start at an opened `/`; relative roots start at an opened process current directory. Every supplied nonempty root component, including `.`, is opened relative to the previous descriptor with `O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC | O_RDONLY`. Root `..` and NUL are rejected before IO. No `canonicalize`, `resolve`, prefix comparison, or lexical containment test authorizes access.

`ContainedRoot::create` explicitly creates **one** new root, using mode 0700 before umask; ancestors must already exist. It admits the parent by the same descriptor walk, uses `mkdirat`, opens the new directory without following links, and synchronizes the new directory and its parent. An existing leaf is a collision. A failed initialization can leave its new directory visible; there is no automatic rollback or creation receipt on error.

`RelativePath` preserves OS bytes. It rejects an absolute path, an empty path, empty slash-separated components (including doubled/trailing slashes), `.`/`..` and NUL. Non-UTF-8 filenames and ordinary user filenames beginning `.writfold-tmp-` remain usable. Root path syntax is not the same as this deliberately stricter descendant grammar.

`pin` walks descendant directories without following links and retains the final parent descriptor plus one leaf name. Subsequent reads, temporaries, publication, and synchronization use that descriptor, never the original ancestor pathname. If the parent is renamed after admission, operations remain attached to the admitted directory inode; they do not switch to whatever occupies its previous path. A root or parent descriptor is an object capability, not a promise that its directory still has the original textual ancestry.

Leaves are opened with `O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK` **before** checking that they are regular files. FIFOs therefore cannot block admission waiting for a peer. Regular-file IO itself can still block in the kernel: byte bounds are not wall-clock/cancellation guarantees. Opening an arbitrary special file is not a device sandbox; access to untrusted device namespaces is outside the qualified profile.

## Exact bounded reads

An `OpenedFile` holds the leaf descriptor and borrows its pinned parent. Reads use `pread`, so multiple reads do not share or advance a file offset. Retained allocation is bounded by the smaller of opened size and the caller's byte limit; allocation failure is explicit. At most that prefix plus one byte of EOF probing is read. Complete empty files and nonempty zero-budget truncations are distinguishable.

Before and after reading, the adapter compares the opened inode's device/inode, size, link count, mode, uid/gid, mtime and ctime (including nanoseconds). It also compares the named leaf, obtained without following links, against the opened stamp. Atime is deliberately excluded because reads may update it. Removal, observed replacement, changed metadata, and size/EOF inconsistency are failures, not complete empty evidence. An IO failure does not return a complete result with partial bytes.

These comparisons are **observations**, not a proof that an uncooperative writer could not perform an ABA change between observations or exploit timestamp granularity. Ordinary reads do not take or create writer locks. Callers needing a cooperating-writer critical section explicitly acquire a managed guard and use its read method.

## Managed coordination, not adjacent lock files

Following plan/11 FR11, `ManagedLocks` admits one separately managed namespace. `open` opens an existing namespace without initializing it; `create` explicitly initializes one new directory. `LockKey` accepts 1–128 ASCII letters, digits, `_` and `-`. An entry is `<key>.lock` inside that namespace. The application supplies the stable logical-record key; it must not derive a different key just because an authored path or content revision changes.

`PinnedFile::try_lock` is an explicit write-capability operation. It creates a missing lock entry with mode 0600 before umask, rejects symlinks/nonregular entries/multiply linked entries, and uses nonblocking exclusive `flock`. Busy returns `LockBusy` without a sleep/retry loop. Successful acquisition checks the retained and named lock inode, then synchronizes the lock file and namespace directory. Revalidation occurs before publication. Dropping the guard explicitly unlocks before closing its descriptor; **it never unlinks the lock entry**. This prevents an unrelated fork's inherited open-file description from extending the logical lock lifetime. Owned effects must already be quiescent before drop; unlock failure cannot be returned from `Drop`, and later acquisition still observes any remaining kernel lock as busy.

All coordinating readers, writers and recovery code must use the same namespace inode and logical key. Two paths may intentionally share one logical-record lock. Conversely, choosing different keys for the same record is **not CAS**. Namespaces cannot be renamed/replaced/unlinked by another participant while clients may reopen them; namespace migration requires quiescence. Python's adjacent `.<leaf>.kctx-lock` is not used and does not interoperate automatically with this managed namespace. Mixed old/new writers require an explicit quiescent cutover, not an implicit alias.

Opening a root, pinning, opening a source, and ordinary reads never initialize directories, locks, recovery state, or ownership. Acquiring a guard is explicitly different from ordinary reading. `ManagedLocks::open` itself also has no creation side effects.

## Atomic publication and uncertainty

Under a retained managed lock, a conditional update reads and compares the full current revision. Current files with a link count other than one are refused, preserving authored hardlink-write refusal. An exceeded current-read budget cannot be used as a revision match. The old leaf descriptor remains open across preparation and the final unchanged check.

Publication uses these ordered effects:

1. Enforce the replacement byte budget and compute the checked content digest.
2. Create a same-directory `.writfold-tmp-<pid>-<sequence>` with `O_EXCL | O_CREAT | O_NOFOLLOW`, mode 0600 before umask. The requested destination name is never used for staging. Existing name collisions are skipped, never removed; the attempt count is bounded.
3. Write all bytes, handling short writes and interruption. A zero-byte write is an IO failure. Fsync the temporary file.
4. Recheck the lock inode and, for replacement, the retained old source identity/metadata.
5. For replacement use `renameat`; for exclusive creation use `linkat`, which refuses a pre-existing destination atomically. No check-then-overwrite substitute is used for creation.
6. Remove the operation-owned temporary link if publication used `linkat`, then fsync the held parent directory.
7. Only then construct `SyncedWrite` with digest, byte length and published inode identity.

A temporary cleanup first compares the named entry with the operation's still-open inode. It never sweeps a prefix, removes pre-existing collision entries, or performs automatic GC. Pre-publication cleanup is best effort and can leave an operation-owned temporary if cleanup itself fails. After publication, cleanup failure is explicitly unsuccessful. The comparison/unlink pair is not an atomic defense against a malicious same-user namespace mutator; the namespace ownership assumption remains necessary.

`WriteError` has two explicit states:

- `NotPublished(FsError)`: this operation has not completed a successful publication syscall. It may have created its managed lock and/or left its own temporary behind. No successful write receipt is supplied.
- `PublishedButDurabilityUnknown { digest, bytes, source }`: publication succeeded, but temporary cleanup or parent synchronization failed. New bytes became visible at the publication point; this does not promise they remain current after later writers. The caller must re-observe/recover rather than assume failure or blindly replay the old predecessor. There is no `SyncedWrite` in this error.

A crash or killed caller can occur between any syscall and its response. An interrupted response is not evidence that publication failed. This adapter does not implement a recovery journal or a multi-file transaction.

## Recoverable single-file staging

`LockedFile::observe_replacement` captures complete bytes and exact device/inode,
full mode, uid/gid, link count, length, signed mtime/ctime seconds and nanoseconds.
`stage_replacement(&mut self, observation, content, limit)` rechecks that observation,
creates and synchronizes a same-parent stage, and returns an exclusively borrowed,
single-use handle. A second live mutator cannot share that guard. The target stays
unchanged until consuming `install`; dropping the stage deliberately retains it.

This profile admits only an existing, singly linked regular file in a private,
run-owned worktree. It preserves ordinary permission bits and ownership. Symlinks,
hardlinks, special mode bits, enumerable ACLs/xattrs and unsupported ownership
configurations refuse. Administrator-installed hidden attributes are excluded by
the profile assumption: unprivileged `listxattr` cannot prove their absence.
Original timestamps are not preserved. Metadata observations do not establish
hostile-writer or inode-reuse immunity.

The owner must durably claim the original effect/intent before staging, persist
the full witness before installation, recheck current authorization/resource
guards, and retain the final receipt after file/directory synchronization.
These filesystem APIs do not implement that ledger or authorize dispatch.
`InstallError::NotInstalled` and `OutcomeUnknown` remain distinct; a missing reply
never implies that installation failed.

Under admitted reservation/quiescence and the same stable lock,
`reconcile_stage(&mut self, witness, limit)` only observes the original attempt.
It confirms the exact staged inode, candidate bytes and admitted metadata after
reopening, allowing only rename-related ctime drift. Equal candidate bytes on a
foreign inode remain `Unknown`. Exact preimage plus exact retained stage permits
only that stage's cleanup and directory synchronization; a foreign stage is never
swept. Reconciliation never installs or replays an effect.

The component regression suite covers stale bytes/metadata, aliasing mutators,
unsupported metadata, interruption before install, lost receipt, forged witness
and equal-byte foreign-inode outcomes. A static-musl adapter consumer separately
replayed the published Splicery recipe through the embedded kernel, matched the
independent CLI candidate, installed into a fresh disposable directory, reopened
and reconciled the exact inode, then refused equal-byte foreign-inode attribution.
`owner-alpha-components-mgmh53f5/native-replay-001.json` retains that execution
with an empty `PATH`; it is not an owner-service, crash-process or alpha pass.

## Immutable retained-object prerequisite

`writfold_store::objects::ObjectStore::{initialize, open, put, get}` implements
the raw retained-byte portion of the experimental realm layout. Initialization
preflights the closed identity schema and all metadata budgets, creates one new
private root with `identity.json`, `objects/sha256/` and `coordination/`, and
refuses existing roots. It does not initialize policy, keys, corpus publication,
native graphs or effect services. This is not an effect-ready realm.

Open binds the actual expected RealmId, exact original identity bytes and inode.
A live handle refuses identity replacement or instance drift. Continuity across
an arbitrary later reopen still requires the caller's separately retained and
qualified expected incarnation; supplied metadata is not current authority.

Put retains raw bytes under the engineering BlobDigest at
`objects/sha256/<first-two-hex>/<remaining-hex>`. Shards are created lazily under
managed coordination. Existing objects must match complete original bytes and
length; they are synchronized in place, never repaired or replaced. Explicit
put also synchronizes the identity binding under the same key as initialization,
so visible identity bytes left by an uncertain initial publication do not
silently become a durable binding. Reads do not perform that settlement.

Get checks realm, identity, privacy, byte budget, complete length and the
domain-separated fingerprint. It does not create directories/lock entries,
fsync, substitute HEAD or return a truncated success. Symlinks, nonregular files,
hardlinks, wrong-owner/private-mode failures and observed namespace drift refuse.
These checks retain the cooperating-owner assumptions below.

The runtime additions are descriptor-relative `ContainedRoot::create_child`,
explicit private admission/locking and `LockedFile::sync_existing`. The latter
requires a complete captured file stamp, rejects a same-byte foreign inode, and
observes file then parent-directory fsync without rewriting the file. It proves
neither past effect application nor permission. `PublishedButDurabilityUnknown`
remains distinct from a successful retained reference.

Evidence under `~/.local/share/writfold/evidence/retained-objects-ej3iov53/`:
`native-001.json` records a real static-musl consumer, running as the operator UID
with `PATH=/nonexistent`, exercising fresh/reopened stores, exact empty/binary/
UTF-8+CRLF bytes, duplicate inode reuse and scope/content/path/privacy refusals.
`unit-003.json` records 421 passing tests with zero failures/ignored tests;
`quality-001.json` passes 255 checks with no skips. `verus-001.json` passes 273
producer checks and 18 semantic mutant families, including the new structural
reference bindings, not filesystem/refinement proofs.

Review found that reopen could forget identity-publication uncertainty; explicit
retention now settles it under the initialization key. The identity-owner
contention regression exercises the ordering. The obsolete literal-layout-lock
test failure is retained in `unit-002.json`; its replacement probes actual
created coordination entries without pinning an internal lock filename.
Injected fsync failures, host power loss and a separate-process simultaneous
first-put campaign are not claimed by this component evidence.

## Native Worktrunk creation

`writfold_runtime::worktrunk` admits the exact v0.81.0 Linux-musl executable
(`1f3d97a5…`) and a separately pinned Git executable. `Source`, `Destinations`
and the borrowed `CreateRequest` identify the private source repository,
destination namespace, fresh leaf/branch and exact Git base. This component is
not an owner run service or permission to edit.

Creation holds permanent managed source/destination leases through preflight,
native execution, independent observation and owned-child cleanup. Before
issuing Worktrunk, it hashes the actual selected commit and complete checkout
tree/blob closure using the admitted Git hashing implementation. Identical
objects are checked once per kind; every checkout path is still budgeted.
Naming an OID or receiving a successful `cat-file` result is insufficient.
It does not scan unrelated commit history.

The profile permits only private, canonical standalone loose-object repositories
with closed configuration. Packed/shallow/alternate repositories, remotes,
includes, filters, attribute files, project Worktrunk configuration, symlinks,
submodules and special files refuse. User/system configuration is isolated;
the child PATH contains only the still-open admitted Git descriptor binding.
Worktrunk and Git hooks are disabled. Filesystem scans check cooperative
cancellation/deadlines as well as name/content/depth limits; this is not a
hard-realtime guarantee for a blocking filesystem syscall.

After a successful native process, Git independently checks the common directory
and native device/inode, object format, exact base, branch and contained path.
Original source bytes/modes and HEAD/branch are checked again. Worktrunk stdout
is retained as a bounded observation, not trusted as the binding.
Pre-issue refusals are `Unissued`; failures after the creation boundary are
`Unknown`, including failed post-observation. Neither a timeout nor nonzero exit
authorizes rollback, deletion or retry. The future owner must retain its durable
claim and unresolved resource reservation independently of these native leases.

Repeat the actual process campaign with an already retained pinned `wt`:

```sh
python3 -B scripts/writfold_worktrunk.py \
  --worktrunk "$WT" \
  --receipt "$HOME/.local/share/writfold/evidence/worktrunk-NEW.json"
```

The producer admits the existing local tools image/storage binding, builds the
static-musl consumer, and runs fresh operator-owned SHA-1 and SHA-256 fixtures.
`worktrunk-runtime-b_b3byph/campaign-001.json` passed normal creation, armed Git
and Worktrunk hook controls, poisoned ambient PATH/configuration, exact nested/
empty/shared-blob checkout, source preservation, conflicts and pre-cancellation.
It also passed corrupt commit, tree and blob controls before creation in both
formats. The executable is
`692290397340826f7e9b926b41646e6ebcce40a7d896b18f73d33a1d0a32b6d1`.

The real failing-before `corrupt-blob-before.json` remains retained: the earlier
adapter returned `Created` for corrupt loose-object bytes. Hash verification
closes that bug; the permanent producer exercises its regressions. This evidence
does not qualify interrupted creation, coordinator-crash child ownership,
reconciliation, signed grants, a full WF gate or an installable alpha.

## Required qualification and trust boundary

The supported target is Linux/WSL x86_64. The intended filesystem profile is an operator-admitted **local Linux filesystem** supporting descriptor-relative no-follow opens, coherent inode/stat observations, exclusive creation, advisory flock, atomic same-directory rename/hardlink publication, and file/directory fsync. Qualification must bind the actual mount/filesystem and storage path used by the operator. NFS, SMB, FUSE, Windows/DrvFS mounts, unusual pseudo-filesystems, and other environments with different locking/rename/error/durability semantics are not implicitly qualified. A Linux target triple alone is not filesystem qualification. Detected unsupported syscalls are errors; the adapter does not silently downgrade.

Additional assumptions:

- Writers and recovery cooperate on one stable managed namespace/key protocol. Same-user malicious memory/filesystem manipulation and arbitrary direct external writes are outside the CAS guarantee.
- The process current directory anchors relative roots. Process-global cwd mutation is not supplied by this adapter. Bind mounts and mount crossing are not rejected; the admitted root and mount topology are operator trust inputs.
- Directory namespace owners do not maliciously replace operation temporaries or lock entries between checking and publication/cleanup. Observed replacements are refused, but metadata checks do not establish lock-free ABA immunity.
- Successful local publication syscalls have the qualified local filesystem's namespace semantics; remote filesystems with ambiguous failed rename/link responses need another outcome protocol.
- Fsync success is an observed OS result, not proof of drive-cache behavior or host-power-loss safety. No power-loss guarantee, automatic GC, device sandbox, or cross-store atomicity is claimed.
- Read byte budgets, write byte budgets and lock contention are explicit; supervisor-level IO deadlines/cancellation and process lifetime management belong outside this module.

## Test and evidence scope

`tests/filesystem.rs` loads isolated real-directory tests for byte-preserving path validation, absolute/relative root symlink ancestors, leaf symlinks, FIFO/nonregular refusal, no implicit initialization, retained roots/parents under rename, observed leaf replacement/in-place changes, zero/partial/full and multi-buffer reads, checked digest examples, conditional creation/replacement, stale revisions, both byte budgets, exclusive-create collision, hardlink refusal, stable managed lock inodes, lock-entry substitution, and a channel-coordinated competing writer. They use no user KB, time-based races, mock syscall echoes or production credentials.

Integration exercised all 24 filesystem regressions within the 66-test Rust run (`rust-unit-005.json`) and a separately compiled static `contained_smoke` consumer on the observed WSL ext4 filesystem with an empty PATH. That consumer created a new isolated record, published a conditional replacement, rejected a stale write, and observed the retained replacement bytes and inode. Evidence is retained under the operator's Writfold qualification root in `native-files-001/`; no user KB was touched. This does not close P05: post-publication IO-failure injection, process-crash windows and full source/Git qualification remain required. No power-loss guarantee follows.
