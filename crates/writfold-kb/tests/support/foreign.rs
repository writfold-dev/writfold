//! The foreign-member check (#190): for a reader and the places of its
//! universe, add one member the reader does not know and assert that the
//! verb's universal word changes or the member is counted. This is the
//! "removing this check fails a test" discipline turned on inputs.
use super::TestResult;

/// For each place in `universe`: a fresh world from `fresh`, one foreign
/// member added there by `add`, the verb run by `read`, and `counted`
/// asked of its text and exit code. Fails naming the first place whose
/// foreign member went unnoticed.
pub fn assert_foreign_counted<W>(
    universe: &[&str],
    fresh: impl Fn() -> TestResult<W>,
    add: impl Fn(&W, &str) -> TestResult,
    read: impl Fn(&W) -> TestResult<(String, u8)>,
    counted: impl Fn(&str, u8) -> bool,
) -> TestResult {
    for place in universe {
        let world = fresh()?;
        add(&world, place)?;
        let (text, code) = read(&world)?;
        if !counted(&text, code) {
            return Err(format!(
                "a foreign member at {place} went unnoticed (exit {code}): {text}"
            )
            .into());
        }
    }
    Ok(())
}
