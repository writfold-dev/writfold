use crate::Error;
use std::fmt::Write;

/// JSON float tokens become Python binary64 values before json.dumps formats
/// them. Rust's shortest scientific formatter supplies the round-tripping
/// significand; Python's repr policy selects fixed/scientific presentation.
pub(super) fn python_float(token: &str, offset: usize) -> Result<String, Error> {
    let unsupported = || Error::UnsupportedNumber { offset };
    let value: f64 = token.parse().map_err(|_| unsupported())?;
    if !value.is_finite() {
        return Err(unsupported());
    }
    let mut text = format!("{value:e}");
    let exponent_at = text.find('e').ok_or_else(unsupported)?;
    let exponent: i32 = text[exponent_at + 1..].parse().map_err(|_| unsupported())?;
    text.truncate(exponent_at);
    if !(-4..16).contains(&exponent) {
        write!(&mut text, "e{exponent:+03}").map_err(|_| unsupported())?;
        return Ok(text);
    }
    let sign = usize::from(text.starts_with('-'));
    if let Some(dot) = text.find('.') {
        text.remove(dot);
    }
    let digits = text.len() - sign;
    let decimal = exponent + 1;
    if decimal <= 0 {
        text.insert_str(sign, "0.");
        for _ in decimal..0 {
            text.insert(sign + 2, '0');
        }
    } else {
        let decimal = usize::try_from(decimal).map_err(|_| unsupported())?;
        if decimal >= digits {
            for _ in digits..decimal {
                text.push('0');
            }
            text.push_str(".0");
        } else {
            text.insert(sign + decimal, '.');
        }
    }
    Ok(text)
}
