//! Pure invocation-local state corresponding to `engine/protocol.py::Engine`.
//! These constructors label identities and state only: no live-process ownership,
//! writer authorization, generation allocation or IO capability is conferred.
//! Adapters validate envelopes and look up methods before consulting this state;
//! unknown methods therefore remain method-not-found even after shutdown.
use vstd::prelude::*;
use writfold_domain::{InstanceId, RequestGeneration, SessionIncarnation};

verus! {
/// Both namespace components are compared exactly and borrowed, not copied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionIdentity<'a> {
    instance: &'a InstanceId,
    incarnation: &'a SessionIncarnation,
}
impl View for SessionIdentity<'_> {
    type V = (Seq<char>, Seq<char>);
    closed spec fn view(&self) -> Self::V { (self.instance@, self.incarnation@) }
}
impl<'a> SessionIdentity<'a> {
    pub fn new(instance: &'a InstanceId, incarnation: &'a SessionIncarnation) -> (value: Self)
        ensures value@ == (instance@, incarnation@),
    { Self { instance, incarnation } }

    pub fn instance(self) -> (value: &'a InstanceId)
        ensures value@ == self@.0,
    { self.instance }

    pub fn incarnation(self) -> (value: &'a SessionIncarnation)
        ensures value@ == self@.1,
    { self.incarnation }

    fn same(self, other: SessionIdentity<'_>) -> (equal: bool)
        ensures equal == (self@ == other@),
    { self.instance == other.instance && self.incarnation == other.incarnation }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestIdentity<'a> {
    session: SessionIdentity<'a>,
    generation: RequestGeneration,
}
impl View for RequestIdentity<'_> {
    type V = ((Seq<char>, Seq<char>), u64);
    closed spec fn view(&self) -> Self::V { (self.session@, self.generation.value()) }
}
impl<'a> RequestIdentity<'a> {
    pub fn new(session: SessionIdentity<'a>, generation: RequestGeneration) -> (value: Self)
        ensures value@ == (session@, generation.value()),
    { Self { session, generation } }

    pub fn session(self) -> (value: SessionIdentity<'a>)
        ensures value@ == self@.0,
    { self.session }

    pub fn generation(self) -> (value: RequestGeneration)
        ensures value.value() == self@.1,
    { self.generation }

    /// Exact correlation only, not proof that the request is live or authorized.
    /// A different incarnation is stale even when generation numbers coincide.
    pub fn ensure_current(self, current: RequestIdentity<'_>)
        -> (result: Result<(), IdentityMismatch>)
        ensures result == if self@.0 != current@.0 { Err(IdentityMismatch::Session) }
            else if self@.1 != current@.1 { Err(IdentityMismatch::Generation) }
            else { Ok(()) },
    {
        if !self.session.same(current.session) {
            Err(IdentityMismatch::Session)
        } else if self.generation.get() != current.generation.get() {
            Err(IdentityMismatch::Generation)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityMismatch { Session, Generation }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionState { Uninitialized, Initialized, ShutDown }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionError { InitializationRequired, AlreadyInitialized, ShutDown }

impl SessionError {
    pub fn code(self) -> (code: i32)
        ensures code == match self {
            Self::AlreadyInitialized => -32000,
            Self::InitializationRequired | Self::ShutDown => -32002,
        },
    {
        match self {
            Self::AlreadyInitialized => -32000,
            Self::InitializationRequired | Self::ShutDown => -32002,
        }
    }
}

/// No clone/copy: transitions have one invocation-local mutable owner.
/// Identity labels confer neither live process ownership nor corpus authority.
#[derive(Debug)]
pub struct Session<'a> {
    identity: SessionIdentity<'a>,
    state: SessionState,
}
impl View for Session<'_> {
    type V = ((Seq<char>, Seq<char>), SessionState);
    closed spec fn view(&self) -> Self::V { (self.identity@, self.state) }
}
impl<'a> Session<'a> {
    pub fn new(identity: SessionIdentity<'a>) -> (value: Self)
        ensures value@ == (identity@, SessionState::Uninitialized),
    { Self { identity, state: SessionState::Uninitialized } }

    pub fn identity(&self) -> (value: SessionIdentity<'a>)
        ensures value@ == self@.0,
    { self.identity }

    pub fn state(&self) -> (value: SessionState)
        ensures value == self@.1,
    { self.state }

    /// Check before method parameters, except for initialize's schema-first rule.
    pub fn require_initialized(&self) -> (result: Result<(), SessionError>)
        ensures result == match self@.1 {
            SessionState::Uninitialized => Err(SessionError::InitializationRequired),
            SessionState::Initialized => Ok(()),
            SessionState::ShutDown => Err(SessionError::ShutDown),
        },
    {
        match self.state {
            SessionState::Uninitialized => Err(SessionError::InitializationRequired),
            SessionState::Initialized => Ok(()),
            SessionState::ShutDown => Err(SessionError::ShutDown),
        }
    }

    /// Check before initialize's schema validation.
    pub fn require_running(&self) -> (result: Result<(), SessionError>)
        ensures result == match self@.1 {
            SessionState::ShutDown => Err(SessionError::ShutDown),
            _ => Ok(()),
        },
    {
        match self.state {
            SessionState::ShutDown => Err(SessionError::ShutDown),
            _ => Ok(()),
        }
    }

    /// Check after initialize's schema validation, before capability admission.
    pub fn require_uninitialized(&self) -> (result: Result<(), SessionError>)
        ensures result == match self@.1 {
            SessionState::Uninitialized => Ok(()),
            SessionState::Initialized => Err(SessionError::AlreadyInitialized),
            SessionState::ShutDown => Err(SessionError::ShutDown),
        },
    {
        match self.state {
            SessionState::Uninitialized => Ok(()),
            SessionState::Initialized => Err(SessionError::AlreadyInitialized),
            SessionState::ShutDown => Err(SessionError::ShutDown),
        }
    }

    /// Commit only after the adapter validates parameters/capabilities and can
    /// represent the reply. Refusal preserves the entire invocation state.
    pub fn initialize(&mut self) -> (result: Result<(), SessionError>)
        ensures
            final(self)@.0 == old(self)@.0,
            result == match old(self)@.1 {
                SessionState::Uninitialized => Ok(()),
                SessionState::Initialized => Err(SessionError::AlreadyInitialized),
                SessionState::ShutDown => Err(SessionError::ShutDown),
            },
            final(self)@.1 == if result.is_ok() { SessionState::Initialized } else { old(self)@.1 },
    {
        self.require_uninitialized()?;
        self.state = SessionState::Initialized;
        Ok(())
    }

    /// Close owned resources before committing shutdown. A stopped session
    /// cannot be initialized again; refusal preserves identity and state.
    pub fn shutdown(&mut self) -> (result: Result<(), SessionError>)
        ensures
            final(self)@.0 == old(self)@.0,
            result == match old(self)@.1 {
                SessionState::Uninitialized => Err(SessionError::InitializationRequired),
                SessionState::Initialized => Ok(()),
                SessionState::ShutDown => Err(SessionError::ShutDown),
            },
            final(self)@.1 == if result.is_ok() { SessionState::ShutDown } else { old(self)@.1 },
    {
        self.require_initialized()?;
        self.state = SessionState::ShutDown;
        Ok(())
    }
}
}
