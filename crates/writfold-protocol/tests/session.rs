use writfold_domain::{InstanceId, RequestGeneration, SessionIncarnation};
use writfold_protocol::session::{
    IdentityMismatch, RequestIdentity, Session, SessionError, SessionIdentity, SessionState,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn initialization_and_shutdown_refusals_preserve_state() -> TestResult {
    let instance: InstanceId = "instance-1".parse()?;
    let incarnation: SessionIncarnation = "session-1".parse()?;
    let identity = SessionIdentity::new(&instance, &incarnation);
    let mut session = Session::new(identity);
    assert_eq!(session.state(), SessionState::Uninitialized);
    assert_eq!(session.require_running(), Ok(()));
    assert_eq!(session.require_uninitialized(), Ok(()));
    assert_eq!(
        session.require_initialized(),
        Err(SessionError::InitializationRequired)
    );
    assert_eq!(
        session.shutdown(),
        Err(SessionError::InitializationRequired)
    );
    assert_eq!(session.state(), SessionState::Uninitialized);
    assert_eq!(session.initialize(), Ok(()));
    assert_eq!(session.state(), SessionState::Initialized);
    assert_eq!(session.require_initialized(), Ok(()));
    assert_eq!(session.require_running(), Ok(()));
    assert_eq!(
        session.require_uninitialized(),
        Err(SessionError::AlreadyInitialized)
    );
    assert_eq!(session.initialize(), Err(SessionError::AlreadyInitialized));
    assert_eq!(session.state(), SessionState::Initialized);
    assert_eq!(session.shutdown(), Ok(()));
    assert_eq!(session.state(), SessionState::ShutDown);
    assert_eq!(session.require_running(), Err(SessionError::ShutDown));
    assert_eq!(session.require_uninitialized(), Err(SessionError::ShutDown));
    assert_eq!(session.require_initialized(), Err(SessionError::ShutDown));
    assert_eq!(session.initialize(), Err(SessionError::ShutDown));
    assert_eq!(session.shutdown(), Err(SessionError::ShutDown));
    assert_eq!(session.state(), SessionState::ShutDown);
    assert_eq!(SessionError::InitializationRequired.code(), -32002);
    assert_eq!(SessionError::AlreadyInitialized.code(), -32000);
    assert_eq!(SessionError::ShutDown.code(), -32002);
    Ok(())
}

#[test]
fn request_correlation_rejects_stale_incarnation_and_generation_exactly() -> TestResult {
    let instance: InstanceId = "instance-1".parse()?;
    let same_instance: InstanceId = "instance-1".parse()?;
    let other_instance: InstanceId = "Instance-1".parse()?;
    let first: SessionIncarnation = "session-1".parse()?;
    let same_first: SessionIncarnation = "session-1".parse()?;
    let next: SessionIncarnation = "session-2".parse()?;
    let generation = RequestGeneration::first();
    let next_generation = generation.checked_next().ok_or("generation exhausted")?;
    let old_session = SessionIdentity::new(&instance, &first);
    let old = RequestIdentity::new(old_session, generation);
    assert_eq!(old.ensure_current(old), Ok(()));
    let equal_value = RequestIdentity::new(
        SessionIdentity::new(&same_instance, &same_first),
        generation,
    );
    assert_eq!(old.ensure_current(equal_value), Ok(()));
    let restarted = RequestIdentity::new(SessionIdentity::new(&instance, &next), generation);
    assert_eq!(
        old.ensure_current(restarted),
        Err(IdentityMismatch::Session)
    );
    assert_eq!(
        restarted.ensure_current(old),
        Err(IdentityMismatch::Session)
    );
    let other = RequestIdentity::new(SessionIdentity::new(&other_instance, &first), generation);
    assert_eq!(old.ensure_current(other), Err(IdentityMismatch::Session));
    let newer = RequestIdentity::new(old_session, next_generation);
    assert_eq!(old.ensure_current(newer), Err(IdentityMismatch::Generation));
    assert_eq!(newer.ensure_current(old), Err(IdentityMismatch::Generation));
    let both_stale = RequestIdentity::new(SessionIdentity::new(&instance, &next), next_generation);
    assert_eq!(
        old.ensure_current(both_stale),
        Err(IdentityMismatch::Session)
    );
    Ok(())
}
