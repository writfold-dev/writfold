use std::{
    collections::TryReserveError,
    fs::File,
    io::{self, Read},
};
use thiserror::Error;
use writfold_domain::{InstanceId, SessionIncarnation};

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("acquiring invocation entropy failed: {0}")]
    Entropy(#[source] io::Error),
    #[error("allocating invocation identity failed: {0}")]
    Allocation(#[from] TryReserveError),
    #[error("invalid invocation identity: {0}")]
    Invalid(#[from] writfold_domain::IdentityError),
}

/// Fresh ephemeral process/session labels, not KB identities or credentials.
///
/// Entropy comes only from the trusted Linux OS. No timestamp, PID, persisted
/// owner, authorization grant, or deterministic fallback is used.
#[derive(Debug)]
pub struct InvocationIdentity {
    instance: InstanceId,
    incarnation: SessionIncarnation,
}

impl InvocationIdentity {
    pub fn new() -> Result<Self, IdentityError> {
        let mut source = File::open("/dev/urandom").map_err(IdentityError::Entropy)?;
        Self::from_entropy(&mut source)
    }

    fn from_entropy(source: &mut impl Read) -> Result<Self, IdentityError> {
        let mut entropy = [0_u8; 32];
        source
            .read_exact(&mut entropy)
            .map_err(IdentityError::Entropy)?;
        Ok(Self {
            instance: InstanceId::try_from(label("instance-", &entropy[..16])?)?,
            incarnation: SessionIncarnation::try_from(label("session-", &entropy[16..])?)?,
        })
    }

    pub fn instance(&self) -> &InstanceId {
        &self.instance
    }

    pub fn incarnation(&self) -> &SessionIncarnation {
        &self.incarnation
    }
}

fn label(prefix: &str, entropy: &[u8]) -> Result<String, IdentityError> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::new();
    value.try_reserve_exact(prefix.len() + entropy.len() * 2)?;
    value.push_str(prefix);
    for byte in entropy {
        value.push(char::from(HEX[usize::from(byte >> 4)]));
        value.push(char::from(HEX[usize::from(byte & 15)]));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_and_incarnation_use_independent_entropy_components() -> Result<(), IdentityError> {
        let first = InvocationIdentity::from_entropy(&mut [0_u8; 32].as_slice())?;
        let mut changed = [0_u8; 32];
        changed[31] = 1;
        let incarnation = InvocationIdentity::from_entropy(&mut changed.as_slice())?;
        assert_eq!(first.instance(), incarnation.instance());
        assert_ne!(first.incarnation(), incarnation.incarnation());
        changed[31] = 0;
        changed[0] = 1;
        let instance = InvocationIdentity::from_entropy(&mut changed.as_slice())?;
        assert_ne!(first.instance(), instance.instance());
        assert_eq!(first.incarnation(), instance.incarnation());
        Ok(())
    }

    #[test]
    fn incomplete_entropy_is_not_an_identity() {
        let result = InvocationIdentity::from_entropy(&mut [0_u8; 31].as_slice());
        assert!(matches!(result, Err(IdentityError::Entropy(error))
            if error.kind() == io::ErrorKind::UnexpectedEof));
    }
}
