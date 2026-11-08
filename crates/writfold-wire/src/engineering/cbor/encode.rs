use super::{Error, Value, integer};
use crate::{Budget, Limits};

pub(crate) fn argument_header(major: u8, argument: u64, header: &mut [u8; 9]) -> usize {
    let (info, width) = match argument {
        0..=23 => (argument as u8, 0),
        24..=255 => (24, 1),
        256..=65_535 => (25, 2),
        65_536..=4_294_967_295 => (26, 4),
        _ => (27, 8),
    };
    header[0] = (major << 5) | info;
    header[1..1 + width].copy_from_slice(&argument.to_be_bytes()[8 - width..]);
    1 + width
}

pub(super) struct Writer {
    pub(super) bytes: Vec<u8>,
    limits: Limits,
}

impl Writer {
    pub(super) fn room(&self, length: usize) -> Result<usize, Error> {
        self.bytes
            .len()
            .checked_add(length)
            .filter(|&end| end <= self.limits.bytes())
            .ok_or_else(|| self.limit(Budget::Bytes))
    }

    fn limit(&self, budget: Budget) -> Error {
        Error::Limit {
            budget,
            offset: self.bytes.len(),
        }
    }

    pub(super) fn push(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.room(bytes.len())?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    pub(super) fn header(&mut self, major: u8, argument: u64) -> Result<(), Error> {
        let mut header = [0_u8; 9];
        let length = argument_header(major, argument, &mut header);
        self.push(&header[..length])
    }

    fn blob(&mut self, major: u8, bytes: &[u8]) -> Result<(), Error> {
        let length = u64::try_from(bytes.len()).map_err(|_| self.limit(Budget::Bytes))?;
        self.header(major, length)?;
        self.push(bytes)
    }

    fn container(
        &mut self,
        major: u8,
        length: usize,
        depth: usize,
        nodes: usize,
    ) -> Result<(), Error> {
        if depth == self.limits.depth() {
            return Err(self.limit(Budget::Depth));
        }
        if length > self.limits.container_entries() {
            return Err(self.limit(Budget::ContainerEntries));
        }
        if length > self.limits.nodes() - nodes {
            return Err(self.limit(Budget::Nodes));
        }
        // Every entry needs at least one byte (two for map key/value pairs).
        let minimum = length
            .checked_mul(if major == 5 { 2 } else { 1 })
            .ok_or_else(|| self.limit(Budget::Bytes))?;
        self.room(minimum)?;
        let length = u64::try_from(length).map_err(|_| self.limit(Budget::ContainerEntries))?;
        self.header(major, length)
    }
}

enum Frame<'v, 'a> {
    Array(std::slice::Iter<'v, Value<'a>>),
    Map(std::vec::IntoIter<&'v (&'a str, Value<'a>)>),
}

impl<'v, 'a> Frame<'v, 'a> {
    fn next(&mut self, writer: &mut Writer) -> Result<Option<&'v Value<'a>>, Error> {
        match self {
            Self::Array(values) => Ok(values.next()),
            Self::Map(entries) => match entries.next() {
                Some((key, value)) => {
                    writer.blob(3, key.as_bytes())?;
                    Ok(Some(value))
                }
                None => Ok(None),
            },
        }
    }
}

pub(super) fn encode(value: &Value<'_>, limits: Limits) -> Result<Vec<u8>, Error> {
    let mut writer = Writer {
        bytes: Vec::new(),
        limits,
    };
    let mut stack = Vec::new();
    let mut current = Some(value);
    let mut nodes = 0_usize;
    loop {
        if let Some(value) = current.take() {
            if nodes == limits.nodes() {
                return Err(writer.limit(Budget::Nodes));
            }
            nodes += 1;
            match value {
                Value::Null => writer.push(&[0xf6])?,
                Value::Bool(value) => writer.push(&[if *value { 0xf5 } else { 0xf4 }])?,
                Value::Unsigned(value) => writer.header(0, *value)?,
                Value::Signed(value) if *value >= 0 => writer.header(0, *value as u64)?,
                Value::Signed(value) => writer.header(1, (-1 - *value) as u64)?,
                Value::Integer(value) => integer::encode(&mut writer, value)?,
                Value::Bytes(bytes) => writer.blob(2, bytes)?,
                Value::Text(text) => writer.blob(3, text.as_bytes())?,
                Value::Array(values) => {
                    writer.container(4, values.len(), stack.len(), nodes)?;
                    stack.push(Frame::Array(values.iter()));
                }
                Value::Map(entries) => {
                    writer.container(5, entries.len(), stack.len(), nodes)?;
                    let mut sorted: Vec<_> = entries.iter().collect();
                    // Shortest text headers order by length, then exact UTF-8 bytes.
                    sorted.sort_unstable_by(|a, b| {
                        a.0.len()
                            .cmp(&b.0.len())
                            .then_with(|| a.0.as_bytes().cmp(b.0.as_bytes()))
                    });
                    if sorted.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                        return Err(Error::DuplicateKey {
                            offset: writer.bytes.len(),
                        });
                    }
                    stack.push(Frame::Map(sorted.into_iter()));
                }
            }
        }
        loop {
            let Some(frame) = stack.last_mut() else {
                return Ok(writer.bytes);
            };
            current = frame.next(&mut writer)?;
            if current.is_some() {
                break;
            }
            stack.pop();
        }
    }
}
