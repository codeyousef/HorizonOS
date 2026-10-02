//! Deterministic transport fixtures; these are not model/provider acceptance.
use std::io::{self, Write};

/// Simulates a peer accepting at most a fixed number of bytes.
pub struct LimitedWriter {
    pub bytes: Vec<u8>,
    remaining: usize,
}

impl LimitedWriter {
    pub fn new(limit: usize) -> Self {
        Self { bytes: Vec::new(), remaining: limit }
    }
}

impl Write for LimitedWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "fixture peer closed"));
        }
        let count = self.remaining.min(data.len());
        self.bytes.extend_from_slice(&data[..count]);
        self.remaining -= count;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
