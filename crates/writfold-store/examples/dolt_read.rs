//! Development-only native qualification consumer; not an installed product CLI.
mod support;

use std::error::Error;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use writfold_domain::dolt::DoltSnapshot;
use writfold_domain::{ByteCount, RowLimit};
use writfold_runtime::dolt::{DoltDatabase, DoltExecutable, QUERY_CONTRACT};
use writfold_runtime::fs::{ContainedRoot, PinnedFile, ReadExtent, RelativePath};
use writfold_runtime::process::ProcessSupervisor;
use writfold_store::dolt::{ReadBudgets, SCHEMA_SQL, read_snapshot};
use writfold_wire::Limits;
use writfold_wire::dolt::{decode_pin, encode_pin};

type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;
const PIN_BYTES: usize = 65_536;
const USAGE: &str = "dolt_read DOLT DB PRIVATE_HOME PIN_JSON [--rows N] [--stdout-bytes N] [--stderr-bytes N] [--timeout-ms N] [--cancelled]; or --schema / --query-contract";

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            // The process already fails; a broken diagnostic pipe cannot turn it into success.
            let _ = support::emit(error.as_ref());
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.len() == 1 {
        let contract = match args[0].to_str() {
            Some("--schema") => SCHEMA_SQL,
            Some("--query-contract") => QUERY_CONTRACT,
            _ => return Err(invalid(USAGE).into()),
        };
        io::stdout().lock().write_all(contract.as_bytes())?;
        return Ok(());
    }
    if args.len() < 4 {
        return Err(invalid(USAGE).into());
    }
    let options = Options::parse(&args[4..])?;
    let stdout = usize::try_from(options.stdout)?;
    let row_probe = usize::try_from(
        options
            .rows
            .checked_add(1)
            .ok_or_else(|| invalid("row limit overflow"))?,
    )?;
    let json_nodes = row_probe
        .checked_mul(6)
        .and_then(|n| n.checked_add(2))
        .ok_or_else(|| invalid("JSON node limit overflow"))?
        .max(32);
    let wire = Limits::new(stdout, 4, row_probe.max(16), json_nodes)?;
    let budgets = ReadBudgets::new(
        ByteCount::new(options.stdout)?,
        ByteCount::new(options.stderr)?,
        RowLimit::new(options.rows)?,
        wire,
        Duration::from_millis(options.timeout_ms),
    );
    let report_limit = stdout
        .checked_mul(2)
        .and_then(|n| n.checked_add(PIN_BYTES + 512))
        .ok_or_else(|| invalid("report byte limit overflow"))?;
    let pin_target = pin_file(Path::new(&args[3]))?;
    let retained = pin_target.read(ByteCount::new(PIN_BYTES as u64)?)?;
    if !matches!(retained.extent(), ReadExtent::Complete) {
        return Err(invalid("pin file exceeds byte limit").into());
    }
    let pin = decode_pin(retained.bytes(), Limits::new(PIN_BYTES, 4, 16, 32)?)?;
    let supervisor = ProcessSupervisor::enable()?;
    let executable_file = pin_file(Path::new(&args[0]))?;
    let executable = DoltExecutable::admit(
        &executable_file,
        pin.producer(),
        ByteCount::new(256 * 1024 * 1024)?,
        &supervisor,
    )?;
    let database = DoltDatabase::open(
        ContainedRoot::open(Path::new(&args[1]))?,
        ContainedRoot::open(Path::new(&args[2]))?,
        ByteCount::new(1024 * 1024)?,
    )?;
    let snapshot = read_snapshot(
        &database,
        &executable,
        pin,
        budgets,
        &AtomicBool::new(options.cancelled),
    )?;
    let report = report(&snapshot, report_limit)?;
    io::stdout().lock().write_all(&report)?;
    Ok(())
}

struct Options {
    rows: u64,
    stdout: u64,
    stderr: u64,
    timeout_ms: u64,
    cancelled: bool,
}

impl Options {
    fn parse(args: &[OsString]) -> Result<Self> {
        let mut result = Self {
            rows: 10_000,
            stdout: 16 * 1024 * 1024,
            stderr: 65_536,
            timeout_ms: 30_000,
            cancelled: false,
        };
        let mut args = args.iter();
        while let Some(flag) = args.next() {
            let flag = flag.to_str().ok_or_else(|| invalid(USAGE))?;
            if flag == "--cancelled" {
                result.cancelled = true;
                continue;
            }
            let destination = match flag {
                "--rows" => &mut result.rows,
                "--stdout-bytes" => &mut result.stdout,
                "--stderr-bytes" => &mut result.stderr,
                "--timeout-ms" => &mut result.timeout_ms,
                _ => return Err(invalid(USAGE).into()),
            };
            *destination = args
                .next()
                .and_then(|value| value.to_str())
                .ok_or_else(|| invalid(USAGE))?
                .parse()?;
        }
        Ok(result)
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn pin_file(path: &Path) -> Result<PinnedFile> {
    let name = path
        .file_name()
        .ok_or_else(|| invalid("missing file name"))?;
    let parent = path.parent().filter(|path| !path.as_os_str().is_empty());
    let root = ContainedRoot::open(parent.unwrap_or_else(|| Path::new(".")))?;
    Ok(root.pin(&RelativePath::new(name)?)?)
}

struct Report {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for Report {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| invalid("report byte limit overflow"))?;
        if length > self.limit {
            return Err(invalid("report byte limit exceeded"));
        }
        self.bytes
            .try_reserve(bytes.len())
            .map_err(io::Error::other)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn json(output: &mut Report, value: &impl serde::Serialize) -> Result<()> {
    serde_json::to_writer(output, value)?;
    Ok(())
}

fn report(snapshot: &DoltSnapshot, limit: usize) -> Result<Vec<u8>> {
    let pin = encode_pin(snapshot.pin(), PIN_BYTES)?;
    let mut output = Report {
        bytes: Vec::new(),
        limit,
    };
    write!(
        output,
        "{{\"edge_count\":{},\"edges\":[",
        snapshot.edges().len()
    )?;
    for (index, edge) in snapshot.edges().iter().enumerate() {
        if index != 0 {
            output.write_all(b",")?;
        }
        output.write_all(b"{\"id\":")?;
        json(&mut output, &edge.id().as_str())?;
        output.write_all(b",\"kind\":")?;
        json(&mut output, &edge.kind().as_str())?;
        output.write_all(b",\"source\":")?;
        json(&mut output, &edge.source().as_str())?;
        output.write_all(b",\"target\":")?;
        json(&mut output, &edge.target().as_str())?;
        output.write_all(b"}")?;
    }
    write!(
        output,
        "],\"node_count\":{},\"nodes\":[",
        snapshot.nodes().len()
    )?;
    for (index, node) in snapshot.nodes().iter().enumerate() {
        if index != 0 {
            output.write_all(b",")?;
        }
        output.write_all(b"{\"home_path\":")?;
        json(&mut output, &node.home_path())?;
        output.write_all(b",\"id\":")?;
        json(&mut output, &node.id().as_str())?;
        output.write_all(b",\"kind\":")?;
        json(&mut output, &node.kind().as_str())?;
        output.write_all(b",\"label\":")?;
        json(&mut output, &node.label())?;
        output.write_all(b"}")?;
    }
    output.write_all(b"],\"pin\":")?;
    output.write_all(&pin)?;
    output.write_all(b"}\n")?;
    Ok(output.bytes)
}
