use std::io::{self, BufRead, BufReader, Read};
use writfold_domain::ByteCount;
use writfold_runtime::stdio::{FrameReadError, FrameReader, InputFrame};

type TestResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;

#[test]
fn exact_bytes_and_boundaries_across_buffer_sizes() -> TestResult {
    for capacity in 1..=12 {
        let input = &b"ab\r\n\n\xff\0\nlast"[..];
        let mut reader = FrameReader::new(
            BufReader::with_capacity(capacity, input),
            ByteCount::new(4)?,
        )?;
        for expected in [&b"ab\r\n"[..], b"\n", b"\xff\0\n", b"last"] {
            assert_eq!(reader.next_frame()?, Some(InputFrame::Bytes(expected)));
        }
        assert_eq!(reader.next_frame()?, None);
        assert_eq!(reader.next_frame()?, None);
    }
    Ok(())
}

#[test]
fn newline_is_included_in_budget_and_drain_stops_at_boundary() -> TestResult {
    for capacity in 1..=16 {
        let input = &b"1234\nOK\n123\r\n\n12345"[..];
        let mut reader = FrameReader::new(
            BufReader::with_capacity(capacity, input),
            ByteCount::new(4)?,
        )?;
        assert_eq!(reader.next_frame()?, Some(InputFrame::Oversized));
        assert_eq!(reader.next_frame()?, Some(InputFrame::Bytes(b"OK\n")));
        assert_eq!(reader.next_frame()?, Some(InputFrame::Oversized));
        assert_eq!(reader.next_frame()?, Some(InputFrame::Bytes(b"\n")));
        assert_eq!(reader.next_frame()?, Some(InputFrame::Oversized));
        assert_eq!(reader.next_frame()?, None);
    }
    Ok(())
}

#[test]
fn empty_input_and_zero_budget() -> TestResult {
    let mut empty = FrameReader::new(&b""[..], ByteCount::new(0)?)?;
    assert_eq!(empty.next_frame()?, None);
    let mut reader = FrameReader::new(&b"\nx\ny"[..], ByteCount::new(0)?)?;
    for _ in 0..3 {
        assert_eq!(reader.next_frame()?, Some(InputFrame::Oversized));
    }
    assert_eq!(reader.next_frame()?, None);
    Ok(())
}

#[test]
fn enormous_budget_does_not_allocate_eagerly() -> TestResult {
    let mut reader = FrameReader::new(&b"x"[..], ByteCount::new(u64::MAX)?)?;
    assert_eq!(reader.next_frame()?, Some(InputFrame::Bytes(b"x")));
    assert_eq!(reader.next_frame()?, None);
    Ok(())
}

// A deterministic BufRead that injects errors between exact input fragments.
// Each error is consumed once; data advances only when consume is called.
enum Step {
    Bytes(&'static [u8]),
    Error(io::ErrorKind),
}

struct Script {
    steps: std::collections::VecDeque<Step>,
}

impl Script {
    fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
        }
    }
}

impl Read for Script {
    fn read(&mut self, target: &mut [u8]) -> io::Result<usize> {
        let available = self.fill_buf()?;
        let length = available.len().min(target.len());
        target[..length].copy_from_slice(&available[..length]);
        self.consume(length);
        Ok(length)
    }
}

impl BufRead for Script {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if let Some(Step::Error(kind)) = self.steps.front() {
            let kind = *kind;
            self.steps.pop_front();
            return Err(io::Error::from(kind));
        }
        Ok(match self.steps.front() {
            Some(Step::Bytes(bytes)) => bytes,
            _ => &[],
        })
    }

    fn consume(&mut self, amount: usize) {
        if let Some(Step::Bytes(bytes)) = self.steps.front_mut() {
            *bytes = &bytes[amount..];
            if bytes.is_empty() {
                self.steps.pop_front();
            }
        }
    }
}

#[test]
fn interrupted_reads_retry_without_losing_or_duplicating_bytes() -> TestResult {
    use Step::{Bytes, Error};
    let source = Script::new([
        Error(io::ErrorKind::Interrupted),
        Bytes(b"ab"),
        Error(io::ErrorKind::Interrupted),
        Error(io::ErrorKind::Interrupted),
        Bytes(b"\nlong"),
        Error(io::ErrorKind::Interrupted),
        Bytes(b"er\n"),
        Error(io::ErrorKind::Interrupted),
        Bytes(b"z"),
        Error(io::ErrorKind::Interrupted),
    ]);
    let mut reader = FrameReader::new(source, ByteCount::new(3)?)?;
    assert_eq!(reader.next_frame()?, Some(InputFrame::Bytes(b"ab\n")));
    assert_eq!(reader.next_frame()?, Some(InputFrame::Oversized));
    assert_eq!(reader.next_frame()?, Some(InputFrame::Bytes(b"z")));
    assert_eq!(reader.next_frame()?, None);
    Ok(())
}

#[test]
fn io_failure_never_returns_partial_or_drained_request() -> TestResult {
    for prefix in [&b"ab"[..], &b"oversized"[..], &b""[..]] {
        let mut steps = Vec::new();
        if !prefix.is_empty() {
            steps.push(Step::Bytes(prefix));
        }
        steps.push(Step::Error(io::ErrorKind::BrokenPipe));
        steps.push(Step::Bytes(b"suffix\nvalid\n"));
        let mut reader = FrameReader::new(Script::new(steps), ByteCount::new(3)?)?;
        assert!(matches!(reader.next_frame(), Err(FrameReadError::Io(error))
            if error.kind() == io::ErrorKind::BrokenPipe));
        assert!(matches!(reader.next_frame(), Err(FrameReadError::Failed)));
    }
    Ok(())
}

#[test]
fn many_oversized_fragments_are_drained_iteratively() -> TestResult {
    let mut steps: Vec<_> = (0..10_000).map(|_| Step::Bytes(b"xxxx")).collect();
    steps.push(Step::Bytes(b"\ny\n"));
    let mut reader = FrameReader::new(Script::new(steps), ByteCount::new(2)?)?;
    assert_eq!(reader.next_frame()?, Some(InputFrame::Oversized));
    assert_eq!(reader.next_frame()?, Some(InputFrame::Bytes(b"y\n")));
    assert_eq!(reader.next_frame()?, None);
    Ok(())
}
