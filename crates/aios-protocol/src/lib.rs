//! Bounded UTF-8 framing. Authentication and JSON/schema validation are separate
//! mandatory layers; accepting a frame does not authorize an action.
use std::io::{self, Read, Write};

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_FRAME_BYTES: usize = 1_048_576;
pub const MAX_TASK_BYTES: usize = 65_536;

pub fn read_frame(reader: &mut impl Read) -> io::Result<Option<String>> {
    let mut header = [0_u8; 4];
    loop {
        match reader.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    reader.read_exact(&mut header[1..])?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid frame length"));
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    String::from_utf8(payload)
        .map(Some)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "frame is not UTF-8"))
}

pub fn write_frame(writer: &mut impl Write, payload: &str) -> io::Result<()> {
    if payload.is_empty() || payload.len() > MAX_FRAME_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid frame length"));
    }
    writer.write_all(&(payload.len() as u32).to_be_bytes())?;
    writer.write_all(payload.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aios_testkit::LimitedWriter;
    use std::io::Cursor;

    #[test]
    fn unicode_frames_and_clean_eof() {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, "{\"text\":\"مرحبا\"}").unwrap();
        let mut reader = Cursor::new(bytes);
        assert_eq!(read_frame(&mut reader).unwrap().unwrap(), "{\"text\":\"مرحبا\"}");
        assert_eq!(read_frame(&mut reader).unwrap(), None);
    }

    #[test]
    fn rejects_length_before_reading_payload() {
        let mut reader = Cursor::new(u32::MAX.to_be_bytes());
        assert_eq!(read_frame(&mut reader).unwrap_err().kind(), io::ErrorKind::InvalidData);
        assert_eq!(reader.position(), 4);
    }

    #[test]
    fn truncated_header_payload_and_invalid_utf8_fail() {
        for bytes in [vec![0], vec![0, 0, 0, 2, b'{']] {
            assert_eq!(read_frame(&mut Cursor::new(bytes)).unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
        }
        assert_eq!(read_frame(&mut Cursor::new(vec![0, 0, 0, 1, 255])).unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn failed_write_never_reports_success() {
        let mut writer = LimitedWriter::new(5);
        assert_eq!(write_frame(&mut writer, "{}").unwrap_err().kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(writer.bytes, vec![0, 0, 0, 2, b'{']);
    }
}
