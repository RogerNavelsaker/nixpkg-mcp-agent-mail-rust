//! MySQL packet writing utilities.
//!
//! This module provides utilities for writing MySQL protocol data types
//! including length-encoded integers and strings.

#![allow(clippy::cast_possible_truncation)]

use crate::protocol::{MAX_PACKET_SIZE, PacketHeader};

/// A writer for MySQL protocol data.
#[derive(Debug, Default)]
pub struct PacketWriter {
    buffer: Vec<u8>,
}

impl PacketWriter {
    /// Create a new writer with default capacity.
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    /// Create a new writer with specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Get the current buffer length.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clear the buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get the buffer as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Consume the writer and return the buffer.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    /// Write a single byte.
    pub fn write_u8(&mut self, value: u8) {
        self.buffer.push(value);
    }

    /// Write a u16 (little-endian).
    pub fn write_u16_le(&mut self, value: u16) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a u24 (little-endian, 3 bytes).
    pub fn write_u24_le(&mut self, value: u32) {
        self.buffer.push((value & 0xFF) as u8);
        self.buffer.push(((value >> 8) & 0xFF) as u8);
        self.buffer.push(((value >> 16) & 0xFF) as u8);
    }

    /// Write a u32 (little-endian).
    pub fn write_u32_le(&mut self, value: u32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a u64 (little-endian).
    pub fn write_u64_le(&mut self, value: u64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a length-encoded integer.
    ///
    /// MySQL uses a variable-length integer encoding:
    /// - 0x00-0xFA: 1-byte value
    /// - 0xFC + 2 bytes: values up to 2^16
    /// - 0xFD + 3 bytes: values up to 2^24
    /// - 0xFE + 8 bytes: values up to 2^64
    pub fn write_lenenc_int(&mut self, value: u64) {
        if value < 251 {
            self.write_u8(value as u8);
        } else if value < 0x10000 {
            self.write_u8(0xFC);
            self.write_u16_le(value as u16);
        } else if value < 0x0100_0000 {
            self.write_u8(0xFD);
            self.write_u24_le(value as u32);
        } else {
            self.write_u8(0xFE);
            self.write_u64_le(value);
        }
    }

    /// Write a length-encoded string.
    pub fn write_lenenc_string(&mut self, s: &str) {
        self.write_lenenc_int(s.len() as u64);
        self.buffer.extend_from_slice(s.as_bytes());
    }

    /// Write a length-encoded byte slice.
    pub fn write_lenenc_bytes(&mut self, data: &[u8]) {
        self.write_lenenc_int(data.len() as u64);
        self.buffer.extend_from_slice(data);
    }

    /// Write a null-terminated string.
    pub fn write_null_string(&mut self, s: &str) {
        self.buffer.extend_from_slice(s.as_bytes());
        self.buffer.push(0);
    }

    /// Write a fixed-length string, padding with zeros if necessary.
    pub fn write_fixed_string(&mut self, s: &str, len: usize) {
        let bytes = s.as_bytes();
        if bytes.len() >= len {
            self.buffer.extend_from_slice(&bytes[..len]);
        } else {
            self.buffer.extend_from_slice(bytes);
            self.buffer.resize(self.buffer.len() + len - bytes.len(), 0);
        }
    }

    /// Write raw bytes.
    pub fn write_bytes(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// Write zeros (padding).
    pub fn write_zeros(&mut self, count: usize) {
        self.buffer.resize(self.buffer.len() + count, 0);
    }

    /// Build a complete packet with header and payload.
    ///
    /// This handles splitting large payloads into multiple packets
    /// if needed (payloads over 16MB - 1).
    pub fn build_packet(&self, sequence_id: u8) -> Vec<u8> {
        self.build_packet_from_payload(&self.buffer, sequence_id)
    }

    /// Build a packet from a given payload.
    pub fn build_packet_from_payload(&self, payload: &[u8], mut sequence_id: u8) -> Vec<u8> {
        let mut result = Vec::with_capacity(payload.len() + 4);

        if payload.len() <= MAX_PACKET_SIZE {
            // Single packet
            let header = PacketHeader {
                payload_length: payload.len() as u32,
                sequence_id,
            };
            result.extend_from_slice(&header.to_bytes());
            result.extend_from_slice(payload);
        } else {
            // Split into multiple packets
            let mut offset = 0;
            while offset < payload.len() {
                let chunk_len = (payload.len() - offset).min(MAX_PACKET_SIZE);
                let header = PacketHeader {
                    payload_length: chunk_len as u32,
                    sequence_id,
                };
                result.extend_from_slice(&header.to_bytes());
                result.extend_from_slice(&payload[offset..offset + chunk_len]);
                offset += chunk_len;
                sequence_id = sequence_id.wrapping_add(1);

                // If we wrote exactly MAX_PACKET_SIZE, we need an empty packet
                // to signal the end of the payload
                if chunk_len == MAX_PACKET_SIZE && offset == payload.len() {
                    let header = PacketHeader {
                        payload_length: 0,
                        sequence_id,
                    };
                    result.extend_from_slice(&header.to_bytes());
                }
            }
        }

        result
    }
}

/// Helper to build a command packet.
pub fn build_command_packet(command: u8, payload: &[u8], sequence_id: u8) -> Vec<u8> {
    let mut writer = PacketWriter::with_capacity(1 + payload.len());
    writer.write_u8(command);
    writer.write_bytes(payload);
    writer.build_packet(sequence_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_u8() {
        let mut writer = PacketWriter::new();
        writer.write_u8(0x42);
        assert_eq!(writer.as_bytes(), &[0x42]);
    }

    #[test]
    fn test_write_u16_le() {
        let mut writer = PacketWriter::new();
        writer.write_u16_le(0x1234);
        assert_eq!(writer.as_bytes(), &[0x34, 0x12]);
    }

    #[test]
    fn test_write_u24_le() {
        let mut writer = PacketWriter::new();
        writer.write_u24_le(0x0012_3456);
        assert_eq!(writer.as_bytes(), &[0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_write_u32_le() {
        let mut writer = PacketWriter::new();
        writer.write_u32_le(0x1234_5678);
        assert_eq!(writer.as_bytes(), &[0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_write_u64_le() {
        let mut writer = PacketWriter::new();
        writer.write_u64_le(0x0807_0605_0403_0201);
        assert_eq!(
            writer.as_bytes(),
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }

    #[test]
    fn test_write_lenenc_int() {
        // 1-byte value
        let mut writer = PacketWriter::new();
        writer.write_lenenc_int(0x42);
        assert_eq!(writer.as_bytes(), &[0x42]);

        // 2-byte value
        let mut writer = PacketWriter::new();
        writer.write_lenenc_int(0x1234);
        assert_eq!(writer.as_bytes(), &[0xFC, 0x34, 0x12]);

        // 3-byte value
        let mut writer = PacketWriter::new();
        writer.write_lenenc_int(0x0012_3456);
        assert_eq!(writer.as_bytes(), &[0xFD, 0x56, 0x34, 0x12]);

        // 8-byte value
        let mut writer = PacketWriter::new();
        writer.write_lenenc_int(0x0807_0605_0403_0201);
        assert_eq!(
            writer.as_bytes(),
            &[0xFE, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }

    #[test]
    fn test_write_null_string() {
        let mut writer = PacketWriter::new();
        writer.write_null_string("hello");
        assert_eq!(writer.as_bytes(), b"hello\0");
    }

    #[test]
    fn test_write_lenenc_string() {
        let mut writer = PacketWriter::new();
        writer.write_lenenc_string("hello");
        assert_eq!(writer.as_bytes(), &[0x05, b'h', b'e', b'l', b'l', b'o']);
    }

    #[test]
    fn test_write_fixed_string() {
        // String shorter than length
        let mut writer = PacketWriter::new();
        writer.write_fixed_string("hi", 5);
        assert_eq!(writer.as_bytes(), &[b'h', b'i', 0, 0, 0]);

        // String exactly matches length
        let mut writer = PacketWriter::new();
        writer.write_fixed_string("hello", 5);
        assert_eq!(writer.as_bytes(), b"hello");

        // String longer than length (truncated)
        let mut writer = PacketWriter::new();
        writer.write_fixed_string("hello world", 5);
        assert_eq!(writer.as_bytes(), b"hello");
    }

    #[test]
    fn test_build_packet() {
        let mut writer = PacketWriter::new();
        writer.write_bytes(b"hello");
        let packet = writer.build_packet(1);
        // Header: 05 00 00 01 + payload: hello
        assert_eq!(&packet[..4], &[0x05, 0x00, 0x00, 0x01]);
        assert_eq!(&packet[4..], b"hello");
    }

    #[test]
    fn test_build_command_packet() {
        let packet = build_command_packet(0x03, b"SELECT 1", 0);
        // Header: 09 00 00 00 + command: 03 + payload: SELECT 1
        assert_eq!(&packet[..4], &[0x09, 0x00, 0x00, 0x00]);
        assert_eq!(packet[4], 0x03);
        assert_eq!(&packet[5..], b"SELECT 1");
    }
}
