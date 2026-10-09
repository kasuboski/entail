//! Framed JSON transport for the persistent author worker.

use std::io::{self, Read, Write};

use serde_json::Value;

pub const DEFAULT_MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// Reads one length-prefixed JSON frame. Clean EOF is `Ok(None)`; a partial
/// header or payload is an I/O error rather than a silently truncated request.
pub fn read_frame(reader: &mut impl Read, max_frame_bytes: usize) -> io::Result<Option<Value>> {
    let mut header = [0_u8; 4];
    let mut filled = 0;
    while filled < header.len() {
        match reader.read(&mut header[filled..]) {
            Ok(0) if filled == 0 => return Ok(None),
            Ok(0) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "partial frame header")),
            Ok(count) => filled += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }

    let length = usize::try_from(u32::from_be_bytes(header))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "frame length cannot be represented"))?;
    if length > max_frame_bytes {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame exceeds configured maximum"));
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    let value = serde_json::from_slice(&payload)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Some(value))
}

/// Writes one JSON value as a complete big-endian length-prefixed frame.
pub fn write_frame(writer: &mut impl Write, value: &Value, max_frame_bytes: usize) -> io::Result<()> {
    let payload = serde_json::to_vec(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if payload.len() > max_frame_bytes || payload.len() > u32::MAX as usize {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "frame exceeds configured maximum"));
    }
    let length = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame length exceeds protocol limit"))?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use serde_json::json;

    use super::{read_frame, write_frame};

    struct Fragmented<'a> {
        bytes: &'a [u8],
        chunk_size: usize,
    }

    impl Read for Fragmented<'_> {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if self.bytes.is_empty() {
                return Ok(0);
            }
            let count = output.len().min(self.chunk_size).min(self.bytes.len());
            output[..count].copy_from_slice(&self.bytes[..count]);
            self.bytes = &self.bytes[count..];
            Ok(count)
        }
    }

    #[test]
    fn fragmented_header_and_payload_are_read_completely() {
        let value = json!({"request_id":"fragmented", "op":"hello"});
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &value, 1024).expect("writing a small frame succeeds");
        let mut input = Fragmented { bytes: &bytes, chunk_size: 1 };
        assert_eq!(read_frame(&mut input, 1024).expect("reading frame succeeds"), Some(value));
        assert_eq!(read_frame(&mut input, 1024).expect("EOF succeeds"), None);
    }

    #[test]
    fn oversized_length_is_rejected_before_allocating_payload() {
        let mut input = Fragmented { bytes: &[0, 0, 0, 9], chunk_size: 4 };
        assert_eq!(read_frame(&mut input, 8).err().map(|error| error.kind()), Some(io::ErrorKind::InvalidData));
    }

    #[test]
    fn partial_header_is_not_clean_eof() {
        let mut input = Fragmented { bytes: &[0, 0], chunk_size: 1 };
        assert_eq!(read_frame(&mut input, 8).err().map(|error| error.kind()), Some(io::ErrorKind::UnexpectedEof));
    }
}
