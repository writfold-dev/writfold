//! Pure invocation-local state corresponding to `engine/protocol.py::Engine`.
//! These constructors label identities and state only: no live-process ownership,
//! writer authorization, generation allocation or IO capability is conferred.
//! Adapters validate envelopes and look up methods before consulting this state;
//! unknown methods therefore remain method-not-found even after shutdown.
use writfold_domain::{InstanceId, RequestGeneration, SessionIncarnation};

/// Both namespace components are compared exactly and borrowed, not copied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionIdentity<'a> {
    instance: &'a InstanceId,
    incarnation: &'a SessionIncarnation,
}

impl<'a> SessionIdentity<'a> {
    pub fn new(instance: &'a InstanceId, incarnation: &'a SessionIncarnation) -> Self {
        Self {
            instance,
            incarnation,
        }
    }

    pub fn instance(self) -> &'a InstanceId {
        self.instance
    }

    pub fn incarnation(self) -> &'a SessionIncarnation {
        self.incarnation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestIdentity<'a> {
    session: SessionIdentity<'a>,
    generation: RequestGeneration,
}

impl<'a> RequestIdentity<'a> {
    pub fn new(session: SessionIdentity<'a>, generation: RequestGeneration) -> Self {
        Self {
            session,
            generation,
        }
    }

    pub fn session(self) -> SessionIdentity<'a> {
        self.session
    }

    pub fn generation(self) -> RequestGeneration {
        self.generation
    }

    /// Exact correlation only, not proof that the request is live or authorized.
    /// A different incarnation is stale even when the instance and generation
    /// are unchanged; ordering or reuse of a numeric generation is irrelevant.
    pub fn ensure_current(self, current: RequestIdentity<'_>) -> Result<(), IdentityMismatch> {
        if self.session != current.session {
            Err(IdentityMismatch::Session)
        } else if self.generation != current.generation {
            Err(IdentityMismatch::Generation)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityMismatch {
    Session,
    Generation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionState {
    Uninitialized,
    Initialized,
    ShutDown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionError {
    InitializationRequired,
    AlreadyInitialized,
    ShutDown,
}

impl SessionError {
    pub fn code(self) -> i32 {
        match self {
            Self::AlreadyInitialized => -32000,
            Self::InitializationRequired | Self::ShutDown => -32002,
        }
    }
}

/// No clone/copy: ordinary state transitions have one local mutable owner.
/// This still makes no claims about other processes or a durable namespace.
#[derive(Debug)]
pub struct Session<'a> {
    identity: SessionIdentity<'a>,
    state: SessionState,
}

impl<'a> Session<'a> {
    pub fn new(identity: SessionIdentity<'a>) -> Self {
        Self {
            identity,
            state: SessionState::Uninitialized,
        }
    }

    pub fn identity(&self) -> SessionIdentity<'a> {
        self.identity
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Check before method-parameter validation (except initialization), matching
    /// the reference's initialization/shutdown refusal precedence.
    pub fn require_initialized(&self) -> Result<(), SessionError> {
        match self.state {
            SessionState::Uninitialized => Err(SessionError::InitializationRequired),
            SessionState::Initialized => Ok(()),
            SessionState::ShutDown => Err(SessionError::ShutDown),
        }
    }

    /// Check before validating initialization parameters. Double-initialization
    /// is checked by `initialize` after schema validation, as in the reference.
    pub fn require_running(&self) -> Result<(), SessionError> {
        match self.state {
            SessionState::ShutDown => Err(SessionError::ShutDown),
            _ => Ok(()),
        }
    }

    /// Check after initialization schema validation, before capability admission.
    pub fn require_uninitialized(&self) -> Result<(), SessionError> {
        match self.state {
            SessionState::Uninitialized => Ok(()),
            SessionState::Initialized => Err(SessionError::AlreadyInitialized),
            SessionState::ShutDown => Err(SessionError::ShutDown),
        }
    }

    /// Commit successful parameter/capability validation. Pure protocol does not
    /// depend on the wire crate; the adapter supplies that separate validation.
    pub fn initialize(&mut self) -> Result<(), SessionError> {
        self.require_uninitialized()?;
        self.state = SessionState::Initialized;
        Ok(())
    }

    /// Commit shutdown after the adapter has completed any owned-resource close.
    /// Refusals never change state, and shutdown cannot be reset by initialization.
    pub fn shutdown(&mut self) -> Result<(), SessionError> {
        self.require_initialized()?;
        self.state = SessionState::ShutDown;
        Ok(())
    }
}
