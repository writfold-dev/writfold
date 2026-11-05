use sha2::{Digest, Sha256};
use writfold_domain::{ContentDigest, IdentityError};

// The contract binds exported schemas by their exact hashes. Its own digest is
// external to those bytes, so it does not contain a circular self-identity.
const CONTRACT: &[u8] = writfold_wire::rpc::response::CONTRACT;

pub(super) fn digest() -> Result<ContentDigest, IdentityError> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(CONTRACT);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 15)]));
    }
    ContentDigest::try_from(encoded)
}
