//! Production conveniences outside the translated surface (Charon starts from
//! `bytes` and `ledger` and never reaches this module).
use crate::ledger::Store;

impl<P> Default for Store<P> {
    fn default() -> Self {
        Store::new()
    }
}
