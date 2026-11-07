use super::admission::Credentials;
use super::http;
use super::request::{BoundedBody, StoredQuery, Target, query_body};
use super::{
    Credential, NativeCall, NativeError, NativeExecutable, NativeLimits, NativeRoot,
    ShutdownObservation, TransportOutcome,
};
use crate::fs::LockedFile;
use crate::process::{ProcessLimits, Service};
use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use writfold_domain::{GraphCommitId, GraphId};

/// Field order is ownership-critical: service Drop quiesces the owned group
/// before credentials/root/executable/coordination lease can be released.
pub struct NativeServer<'a> {
    service: Service<'a>,
    credentials: Credentials,
    root: NativeRoot,
    executable: &'a NativeExecutable<'a>,
    lease: LockedFile<'a>,
    address: SocketAddr,
    stdout_scanned: usize,
    limits: NativeLimits,
    contained: bool,
}

impl<'a> NativeServer<'a> {
    /// Start only the admitted ELF against its fixed root. `credential_directory`
    /// must not exist: this creates private, descriptor-bound credentials once.
    /// The moved coordination lease remains held through shutdown or Drop.
    pub fn start(
        executable: &'a NativeExecutable<'a>,
        root: NativeRoot,
        lease: LockedFile<'a>,
        credential_directory: &Path,
        credentials: Vec<Credential>,
        limits: NativeLimits,
        call: NativeCall<'_>,
    ) -> Result<Self, NativeError> {
        let NativeCall {
            deadline,
            cancelled,
        } = call;
        http::stop(deadline, cancelled)?;
        lease.check_current()?;
        let credentials = Credentials::create(credential_directory, credentials)?;
        let mut command = executable.command(&root)?;
        let grace = limits.shutdown_grace.as_secs();
        if limits.shutdown_grace.subsec_nanos() != 0 || grace == 0 || grace > 300 {
            return Err(NativeError::Input(
                "shutdown grace must be 1..300 whole seconds",
            ));
        }
        command
            .args([
                "--cluster",
                root.canonical_uri(),
                "--bind",
                "127.0.0.1:0",
                "--require-all-graphs",
                "--shutdown-grace-seconds",
                &grace.to_string(),
            ])
            .env("OMNIGRAPH_SERVER_BEARER_TOKENS_FILE", credentials.path());
        credentials.check()?;
        let service = Service::spawn(
            &mut command,
            ProcessLimits::new(limits.stdout, limits.stderr),
            executable.supervisor,
            deadline,
            cancelled,
        )?;
        let mut server = Self {
            service,
            credentials,
            root,
            executable,
            lease,
            address: SocketAddr::from(([127, 0, 0, 1], 0)),
            stdout_scanned: 0,
            limits,
            contained: false,
        };
        loop {
            server.check(deadline, cancelled)?;
            if server.address.port() != 0 {
                return Ok(server);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    pub fn canonical_uri(&self) -> &str {
        self.root.canonical_uri()
    }
    pub fn pid(&self) -> u32 {
        self.service.pid()
    }

    fn check(&mut self, deadline: Instant, cancelled: &AtomicBool) -> Result<(), NativeError> {
        http::stop(deadline, cancelled)?;
        if self.contained {
            return Err(NativeError::Protocol("native owner is contained"));
        }
        match self.check_bindings() {
            Ok(()) => Ok(()),
            Err(cause) => {
                self.contained = true;
                match self.service.contain() {
                    Ok(()) => Err(cause),
                    Err(cleanup) => Err(NativeError::Containment {
                        cause: Box::new(cause),
                        cleanup,
                    }),
                }
            }
        }
    }

    fn check_bindings(&mut self) -> Result<(), NativeError> {
        self.executable.check()?;
        self.root.check()?;
        self.lease.check_current()?;
        self.credentials.check()?;
        if !self.service.pump()? {
            return Err(NativeError::Exited);
        }
        let pending = &self.service.stdout()[self.stdout_scanned..];
        if let Some(last_newline) = pending.iter().rposition(|byte| *byte == b'\n') {
            let consumed = last_newline + 1;
            if let Some(address) = http::announcement(&pending[..consumed])? {
                if self.address.port() != 0 {
                    return Err(NativeError::Protocol("duplicate listener announcement"));
                }
                self.address = address;
            }
            self.stdout_scanned += consumed;
        }
        Ok(())
    }

    /// Named, exact-snapshot read. The boundary always supplies
    /// `expect_mutation:false`; the caller cannot smuggle a mutation envelope.
    pub fn pinned_query(
        &mut self,
        query: StoredQuery<'_>,
        pin: &GraphCommitId,
        call: NativeCall<'_>,
    ) -> TransportOutcome {
        self.query(query, Target::Snapshot(pin), None, call)
    }

    /// Named branch read for initial authenticated readiness/head observation.
    /// The returned head is not pinned until wire semantics validate it.
    pub fn branch_query(
        &mut self,
        query: StoredQuery<'_>,
        branch: &str,
        call: NativeCall<'_>,
    ) -> TransportOutcome {
        self.query(query, Target::Branch(branch), None, call)
    }

    /// One named mutation with the exact native graph-head precondition. No
    /// fallback endpoint or automatic reissue exists, even for HTTP 4xx/5xx.
    pub fn conditional_query(
        &mut self,
        query: StoredQuery<'_>,
        branch: &str,
        expected: &GraphCommitId,
        call: NativeCall<'_>,
    ) -> TransportOutcome {
        self.query(query, Target::Branch(branch), Some(expected), call)
    }

    fn query(
        &mut self,
        query: StoredQuery<'_>,
        target: Target<'_>,
        expected: Option<&GraphCommitId>,
        call: NativeCall<'_>,
    ) -> TransportOutcome {
        let StoredQuery {
            actor,
            graph,
            name,
            params,
        } = query;
        let NativeCall {
            deadline,
            cancelled,
        } = call;
        let prepared = (|| {
            let graph = http::segment(graph.as_str())?;
            let query = http::segment(name)?;
            let path = format!(
                "/graphs/{graph}/queries/{query}{}",
                if expected.is_some() {
                    "/if-graph-commit"
                } else {
                    ""
                }
            );
            let body = query_body(
                target,
                params,
                expected.is_some(),
                self.limits.request_body.get(),
                deadline,
                cancelled,
            )?;
            Ok((path, body))
        })();
        match prepared {
            Err(error) => TransportOutcome::Unissued(error),
            Ok((path, body)) => self.request(actor, "POST", &path, &body, expected, call),
        }
    }

    /// Retain an exact published commit in a new branch. This is effectful,
    /// unlike reads, but never resolves a moving branch as its source.
    pub fn retain_branch(
        &mut self,
        actor: &str,
        graph: &GraphId,
        name: &str,
        from: &GraphCommitId,
        call: NativeCall<'_>,
    ) -> TransportOutcome {
        let NativeCall {
            deadline,
            cancelled,
        } = call;
        let prepared = (|| {
            http::segment(name)?;
            let path = format!("/graphs/{}/branches", http::segment(graph.as_str())?);
            let mut body = BoundedBody::new(self.limits.request_body.get(), deadline, cancelled);
            body.write_all(b"{\"name\":")?;
            serde_json::to_writer(&mut body, name)?;
            body.write_all(b",\"from_commit\":")?;
            serde_json::to_writer(&mut body, from.as_str())?;
            body.write_all(b"}")?;
            Ok((path, body.bytes))
        })();
        match prepared {
            Err(error) => TransportOutcome::Unissued(error),
            Ok((path, body)) => self.request(actor, "POST", &path, &body, None, call),
        }
    }

    /// Exactly one bounded history page. Pagination is explicit, never retried.
    pub fn history(
        &mut self,
        actor: &str,
        graph: &GraphId,
        branch: &str,
        limit: u16,
        page: Option<&str>,
        call: NativeCall<'_>,
    ) -> TransportOutcome {
        let path = (|| {
            if !(1..=1000).contains(&limit) {
                return Err(NativeError::Input("history limit must be 1..1000"));
            }
            let mut path = format!(
                "/graphs/{}/commits?branch={}&limit={limit}",
                http::segment(graph.as_str())?,
                http::segment(branch)?
            );
            if let Some(page) = page {
                path.push_str("&page_token=");
                path.push_str(&http::segment(page)?);
            }
            Ok(path)
        })();
        match path {
            Err(error) => TransportOutcome::Unissued(error),
            Ok(path) => self.request(actor, "GET", &path, b"", None, call),
        }
    }

    pub fn inventory(&mut self, actor: &str, call: NativeCall<'_>) -> TransportOutcome {
        self.request(actor, "GET", "/graphs", b"", None, call)
    }

    fn request(
        &mut self,
        actor: &str,
        method: &str,
        path: &str,
        body: &[u8],
        expected: Option<&GraphCommitId>,
        call: NativeCall<'_>,
    ) -> TransportOutcome {
        let NativeCall {
            deadline,
            cancelled,
        } = call;
        let prepared = (|| {
            self.check(deadline, cancelled)?;
            if body.len() as u64 > self.limits.request_body.get() {
                return Err(NativeError::Budget("request body"));
            }
            let token = self.credentials.token(actor)?;
            let mut header =
                BoundedBody::new(self.limits.request_header.get(), deadline, cancelled);
            write!(
                &mut header,
                "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nOmnigraph-Http-Api: 0.14\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
                self.address,
                body.len()
            )?;
            if let Some(expected) = expected {
                write!(
                    &mut header,
                    "Omnigraph-If-Graph-Commit: {}\r\n",
                    expected.as_str()
                )?;
            }
            header.write_all(b"\r\n")?;
            let socket = http::connect(self.address, deadline, cancelled)?;
            self.check(deadline, cancelled)?;
            Ok((socket, header.bytes))
        })();
        let (mut socket, header) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => return TransportOutcome::Unissued(error),
        };
        // From this point classify every failure conservatively as Unknown.
        match http::exchange(&mut socket, &header, body, self.limits, || {
            self.check(deadline, cancelled)
        }) {
            Ok(response) => TransportOutcome::Observed(response),
            Err(error) => TransportOutcome::Unknown(error),
        }
    }

    /// Teardown never revises an earlier Unknown into a no-effect result.
    /// An error returns while this owner and its coordination lease remain held.
    /// Drop may block beyond the deadline rather than release unquiesced work.
    pub fn shutdown(&mut self, deadline: Instant) -> Result<ShutdownObservation, NativeError> {
        let output = self.service.finish(self.limits.shutdown_grace, deadline)?;
        Ok(ShutdownObservation {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
            forced_kill: output.forced_kill,
        })
    }
}
