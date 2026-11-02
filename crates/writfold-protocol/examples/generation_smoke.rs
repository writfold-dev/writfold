use std::process::ExitCode;
use writfold_domain::RequestGeneration;
use writfold_protocol::generation::{GenerationAdvance, advance_generation};

fn generation(value: std::ffi::OsString) -> Result<RequestGeneration, &'static str> {
    value
        .to_str()
        .and_then(|text| text.parse::<u64>().ok())
        .map(RequestGeneration::from_u64)
        .ok_or("generations must be unsigned 64-bit integers")
}

fn exercise() -> Result<(), &'static str> {
    let mut arguments = std::env::args_os().skip(1);
    let previous = generation(arguments.next().ok_or("supply a previous generation")?)?;
    let active = arguments.map(generation).collect::<Result<Vec<_>, _>>()?;
    match advance_generation(previous, &active) {
        GenerationAdvance::Allocated(next) => {
            println!(
                "{{\"outcome\":\"allocated\",\"generation\":{}}}",
                next.get()
            );
        }
        GenerationAdvance::InUse => println!("{{\"outcome\":\"in_use\"}}"),
        GenerationAdvance::Exhausted => println!("{{\"outcome\":\"exhausted\"}}"),
    }
    Ok(())
}

fn main() -> ExitCode {
    match exercise() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
