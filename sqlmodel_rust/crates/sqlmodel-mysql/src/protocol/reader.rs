//! MySQL packet reading utilities.
//!
//! This module provides utilities for reading MySQL protocol data types
//! including length-encoded integers and strings.

#![allow(clippy::cast_possible_truncation)]

use crate::protocol::{EofPacket, ErrPacket, OkPacket, PacketHeader};

/// A reader for MySQL protocol data.
#[derive(Debug)]
pub struct PacketReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> PacketReader<'a> {
    /// Create a new reader from a byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Get remaining bytes in the buffer.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Check if we've reached the end of the data.
    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Peek at the next byte without advancing.
    pub fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Read a single byte.
    pub fn read_u8(&mut self) -> Option<u8> {
        let byte = self.data.get(self.pos)?;
        self.pos += 1;
        Some(*byte)
    }

    /// Read a u16 (little-endian).
    pub fn read_u16_le(&mut self) -> Option<u16> {
        if self.remaining() < 2 {
            return None;
        }
        let value = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Some(value)
    }

    /// Read a u24 (little-endian, 3 bytes).
    pub fn read_u24_le(&mut self) -> Option<u32> {
        if self.remaining() < 3 {
            return None;
        }
        let value = u32::from(self.data[self.pos])
            | (u32::from(self.data[self.pos + 1]) << 8)
            | (u32::from(self.data[self.pos + 2]) << 16);
        self.pos += 3;
        Some(value)
    }

    /// Read a u32 (little-endian).
    pub fn read_u32_le(&mut self) -> Option<u32> {
        if self.remaining() < 4 {
            return None;
        }
        let value = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Some(value)
    }

    /// Read a u64 (little-endian).
    pub fn read_u64_le(&mut self) -> Option<u64> {
        if self.remaining() < 8 {
            return None;
        }
        let value = u64::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ]);
        self.pos += 8;
        Some(value)
    }

    /// Read a length-encoded integer.
    ///
    /// MySQL uses a variable-length integer encoding:
    /// - 0x00-0xFA: 1-byte value
    /// - 0xFC: 2-byte value follows
    /// - 0xFD: 3-byte value follows
    /// - 0xFE: 8-byte value follows
    /// - 0xFB: NULL (special case for length-encoded strings)
    pub fn read_lenenc_int(&mut self) -> Option<u64> {
        let first = self.read_u8()?;
        match first {
            0x00..=0xFA => Some(u64::from(first)),
            0xFC => self.read_u16_le().map(u64::from),
            0xFD => self.read_u24_le().map(u64::from),
            0xFE => self.read_u64_le(),
            0xFB => None, // NULL marker
            0xFF => None, // Reserved/error
        }
    }

    /// Read a length-encoded string.
    pub fn read_lenenc_string(&mut self) -> Option<String> {
        let len = self.read_lenenc_int()? as usize;
        self.read_string(len)
    }

    /// Read a length-encoded byte slice.
    pub fn read_lenenc_bytes(&mut self) -> Option<Vec<u8>> {
        let len = self.read_lenenc_int()? as usize;
        self.read_bytes(len).map(|b| b.to_vec())
    }

    /// Read a null-terminated string.
    pub fn read_null_string(&mut self) -> Option<String> {
        let start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != 0 {
            self.pos += 1;
        }
        let s = String::from_utf8_lossy(&self.data[start..self.pos]).into_owned();
        // Skip the null terminator
        if self.pos < self.data.len() {
            self.pos += 1;
        }
        Some(s)
    }

    /// Read a fixed-length string.
    pub fn read_string(&mut self, len: usize) -> Option<String> {
        let bytes = self.read_bytes(len)?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }

    /// Read remaining data as a string.
    pub fn read_rest_string(&mut self) -> String {
        let s = String::from_utf8_lossy(&self.data[self.pos..]).into_owned();
        self.pos = self.data.len();
        s
    }

    /// Read a fixed number of bytes.
    pub fn read_bytes(&mut self, len: usize) -> Option<&'a [u8]> {
        if self.remaining() < len {
            return None;
        }
        let bytes = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Some(bytes)
    }

    /// Read remaining bytes.
    pub fn read_rest(&mut self) -> &'a [u8] {
        let rest = &self.data[self.pos..];
        self.pos = self.data.len();
        rest
    }

    /// Skip a number of bytes.
    pub fn skip(&mut self, n: usize) -> bool {
        if self.remaining() >= n {
            self.pos += n;
            true
        } else {
            false
        }
    }

    /// Read a packet header from raw bytes.
    pub fn read_packet_header(&mut self) -> Option<PacketHeader> {
        if self.remaining() < 4 {
            return None;
        }
        let mut header_bytes = [0u8; 4];
        header_bytes.copy_from_slice(&self.data[self.pos..self.pos + 4]);
        self.pos += 4;
        Some(PacketHeader::from_bytes(&header_bytes))
    }

    /// Parse an OK packet from the current position.
    ///
    /// OK packet format (protocol 4.1+):
    /// - 0x00 header (already consumed)
    /// - affected_rows: lenenc int
    /// - last_insert_id: lenenc int
    /// - status_flags: 2 bytes
    /// - warnings: 2 bytes
    /// - info: rest of packet (optional)
    pub fn parse_ok_packet(&mut self) -> Option<OkPacket> {
        // Skip the 0x00 marker if present
        if self.peek() == Some(0x00) {
            self.skip(1);
        }

        let affected_rows = self.read_lenenc_int()?;
        let last_insert_id = self.read_lenenc_int()?;
        let status_flags = self.read_u16_le()?;
        let warnings = self.read_u16_le()?;
        let info = if self.remaining() > 0 {
            self.read_rest_string()
        } else {
            String::new()
        };

        Some(OkPacket {
            affected_rows,
            last_insert_id,
            status_flags,
            warnings,
            info,
        })
    }

    /// Parse an Error packet from the current position.
    ///
    /// ERR packet format (protocol 4.1+):
    /// - 0xFF header (already consumed)
    /// - error_code: 2 bytes
    /// - '#' marker
    /// - sql_state: 5 bytes
    /// - error_message: rest of packet
    pub fn parse_err_packet(&mut self) -> Option<ErrPacket> {
        // Skip the 0xFF marker if present
        if self.peek() == Some(0xFF) {
            self.skip(1);
        }

        let error_code = self.read_u16_le()?;

        // Check for '#' marker (SQL state follows)
        let sql_state = if self.peek() == Some(b'#') {
            self.skip(1);
            self.read_string(5)?
        } else {
            String::new()
        };

        let error_message = self.read_rest_string();

        Some(ErrPacket {
            error_code,
            sql_state,
            error_message,
        })
    }

    /// Parse an EOF packet from the current position.
    ///
    /// EOF packet format:
    /// - 0xFE header (already consumed)
    /// - warnings: 2 bytes
    /// - status_flags: 2 bytes
    pub fn parse_eof_packet(&mut self) -> Option<EofPacket> {
        // Skip the 0xFE marker if present
        if self.peek() == Some(0xFE) {
            self.skip(1);
        }

        let warnings = self.read_u16_le()?;
        let status_flags = self.read_u16_le()?;

        Some(EofPacket {
            warnings,
            status_flags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u8() {
        let mut reader = PacketReader::new(&[0x42, 0x43]);
        assert_eq!(reader.read_u8(), Some(0x42));
        assert_eq!(reader.read_u8(), Some(0x43));
        assert_eq!(reader.read_u8(), None);
    }

    #[test]
    fn test_read_u16_le() {
        let mut reader = PacketReader::new(&[0x34, 0x12]);
        assert_eq!(reader.read_u16_le(), Some(0x1234));
    }

    #[test]
    fn test_read_u24_le() {
        let mut reader = PacketReader::new(&[0x56, 0x34, 0x12]);
        assert_eq!(reader.read_u24_le(), Some(0x0012_3456));
    }

    #[test]
    fn test_read_u32_le() {
        let mut reader = PacketReader::new(&[0x78, 0x56, 0x34, 0x12]);
        assert_eq!(reader.read_u32_le(), Some(0x1234_5678));
    }

    #[test]
    fn test_read_u64_le() {
        let mut reader = PacketReader::new(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        assert_eq!(reader.read_u64_le(), Some(0x0807_0605_0403_0201));
    }

    #[test]
    fn test_read_lenenc_int() {
        // 1-byte value
        let mut reader = PacketReader::new(&[0x42]);
        assert_eq!(reader.read_lenenc_int(), Some(0x42));

        // 2-byte value
        let mut reader = PacketReader::new(&[0xFC, 0x34, 0x12]);
        assert_eq!(reader.read_lenenc_int(), Some(0x1234));

        // 3-byte value
        let mut reader = PacketReader::new(&[0xFD, 0x56, 0x34, 0x12]);
        assert_eq!(reader.read_lenenc_int(), Some(0x0012_3456));

        // 8-byte value
        let mut reader = PacketReader::new(&[0xFE, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        assert_eq!(reader.read_lenenc_int(), Some(0x0807_0605_0403_0201));
    }

    #[test]
    fn test_read_null_string() {
        let mut reader = PacketReader::new(b"hello\0world\0");
        assert_eq!(reader.read_null_string(), Some("hello".to_string()));
        assert_eq!(reader.read_null_string(), Some("world".to_string()));
    }

    #[test]
    fn test_read_lenenc_string() {
        // Length-prefixed string
        let mut reader = PacketReader::new(&[0x05, b'h', b'e', b'l', b'l', b'o']);
        assert_eq!(reader.read_lenenc_string(), Some("hello".to_string()));
    }

    #[test]
    fn test_parse_ok_packet() {
        // OK packet: affected_rows=1, last_insert_id=42, status=2, warnings=0
        let data = [0x00, 0x01, 0x2A, 0x02, 0x00, 0x00, 0x00];
        let mut reader = PacketReader::new(&data);
        let ok = reader.parse_ok_packet().unwrap();
        assert_eq!(ok.affected_rows, 1);
        assert_eq!(ok.last_insert_id, 42);
        assert_eq!(ok.status_flags, 2);
        assert_eq!(ok.warnings, 0);
    }

    #[test]
    fn test_parse_err_packet() {
        // ERR packet: error_code=1045, sql_state=28000, message="Access denied"
        let mut data = vec![0xFF, 0x15, 0x04, b'#'];
        data.extend_from_slice(b"28000");
        data.extend_from_slice(b"Access denied");
        let mut reader = PacketReader::new(&data);
        let err = reader.parse_err_packet().unwrap();
        assert_eq!(err.error_code, 1045);
        assert_eq!(err.sql_state, "28000");
        assert_eq!(err.error_message, "Access denied");
    }

    #[test]
    fn test_parse_eof_packet() {
        // EOF packet: warnings=0, status=2
        let data = [0xFE, 0x00, 0x00, 0x02, 0x00];
        let mut reader = PacketReader::new(&data);
        let eof = reader.parse_eof_packet().unwrap();
        assert_eq!(eof.warnings, 0);
        assert_eq!(eof.status_flags, 2);
    }
}
