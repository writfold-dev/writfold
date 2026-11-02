//! Development conformance executable; not a production RPC or authority adapter.
use std::io::{self, BufRead, Write};
use writfold_wire::reference::{MAX_LINE_BYTES, evaluate_json, invalid_response};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut line = Vec::new();
    loop {
        line.clear();
        let mut overflow = false;
        let mut seen = false;
        loop {
            let bytes = input.fill_buf()?;
            if bytes.is_empty() {
                break;
            }
            seen = true;
            let newline = bytes.iter().position(|byte| *byte == b'\n');
            let count = newline.map_or(bytes.len(), |index| index + 1);
            if !overflow {
                if count > MAX_LINE_BYTES.saturating_sub(line.len()) {
                    overflow = true;
                    line.clear();
                } else {
                    line.extend_from_slice(&bytes[..count]);
                }
            }
            input.consume(count);
            if newline.is_some() {
                break;
            }
        }
        if !seen {
            break;
        }
        if !overflow && line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let response = if overflow {
            invalid_response("")
        } else {
            evaluate_json(&line)
        };
        serde_json::to_writer(&mut output, &response).map_err(io::Error::other)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}
