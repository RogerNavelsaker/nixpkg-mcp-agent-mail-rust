//! PostgreSQL type system and type conversion.
//!
//! This module provides:
//! - OID constants for PostgreSQL built-in types
//! - Encoding/decoding between Rust types and PostgreSQL wire format
//! - Type registry for runtime type information
//!
//! # Example
//!
//! ```rust,ignore
//! use sqlmodel_postgres::types::{oid, encode::Format, decode::decode_value};
//!
//! // Decode a binary integer from PostgreSQL
//! let value = decode_value(oid::INT4, Some(&[0, 0, 0, 42]), Format::Binary)?;
//! assert_eq!(value, Value::Int(42));
//! ```

pub mod decode;
pub mod encode;
pub mod oid;

use std::collections::HashMap;

pub use decode::{BinaryDecode, Decode, TextDecode, decode_value};
pub use encode::{BinaryEncode, Encode, Format, TextEncode, encode_value};

/// Category of a PostgreSQL type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCategory {
    /// Boolean types (bool)
    Boolean,
    /// Numeric types (int2, int4, int8, float4, float8, numeric)
    Numeric,
    /// String types (text, varchar, char, name)
    String,
    /// Date/time types (date, time, timestamp, interval)
    DateTime,
    /// Binary types (bytea)
    Binary,
    /// JSON types (json, jsonb)
    Json,
    /// UUID type
    Uuid,
    /// Array types
    Array,
    /// Range types
    Range,
    /// Composite/record types
    Composite,
    /// Network address types (inet, cidr, macaddr)
    Network,
    /// Geometric types (point, line, circle, etc.)
    Geometric,
    /// Unknown or custom types
    Unknown,
}

/// Information about a PostgreSQL type.
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// The type's OID
    pub oid: u32,
    /// The type's name (e.g., "int4", "text")
    pub name: &'static str,
    /// OID of the array type for this element type (if any)
    pub array_oid: Option<u32>,
    /// OID of the element type (if this is an array type)
    pub element_oid: Option<u32>,
    /// Type category
    pub category: TypeCategory,
    /// Size in bytes (-1 for variable, -2 for null-terminated)
    pub size: i16,
    /// Whether this type supports binary format
    pub binary_format: bool,
}

impl TypeInfo {
    /// Create a new type info.
    const fn new(
        oid: u32,
        name: &'static str,
        category: TypeCategory,
        size: i16,
        array_oid: Option<u32>,
    ) -> Self {
        Self {
            oid,
            name,
            array_oid,
            element_oid: None,
            category,
            size,
            binary_format: true,
        }
    }

    /// Create type info for an array type.
    const fn array(oid: u32, name: &'static str, element_oid: u32) -> Self {
        Self {
            oid,
            name,
            array_oid: None,
            element_oid: Some(element_oid),
            category: TypeCategory::Array,
            size: -1,
            binary_format: true,
        }
    }
}

/// Registry of PostgreSQL types.
///
/// Provides lookup by OID or name for type information.
pub struct TypeRegistry {
    by_oid: HashMap<u32, TypeInfo>,
    by_name: HashMap<&'static str, u32>,
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRegistry {
    /// Create a new type registry with all built-in types.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            by_oid: HashMap::new(),
            by_name: HashMap::new(),
        };

        // Register all built-in types
        registry.register_builtins();

        registry
    }

    /// Look up type info by OID.
    #[must_use]
    pub fn get(&self, oid: u32) -> Option<&TypeInfo> {
        self.by_oid.get(&oid)
    }

    /// Look up type info by name.
    #[must_use]
    pub fn by_name(&self, name: &str) -> Option<&TypeInfo> {
        self.by_name.get(name).and_then(|oid| self.by_oid.get(oid))
    }

    /// Get the type category for an OID.
    #[must_use]
    pub fn category(&self, oid: u32) -> TypeCategory {
        self.get(oid).map_or(TypeCategory::Unknown, |t| t.category)
    }

    /// Check if a type supports binary format.
    #[must_use]
    pub fn supports_binary(&self, oid: u32) -> bool {
        self.get(oid).is_some_and(|t| t.binary_format)
    }

    /// Register a custom type.
    pub fn register(&mut self, info: TypeInfo) {
        self.by_name.insert(info.name, info.oid);
        self.by_oid.insert(info.oid, info);
    }

    fn register_builtins(&mut self) {
        // Boolean
        self.register(TypeInfo::new(
            oid::BOOL,
            "bool",
            TypeCategory::Boolean,
            1,
            Some(oid::BOOL_ARRAY),
        ));

        // Integers
        self.register(TypeInfo::new(
            oid::INT2,
            "int2",
            TypeCategory::Numeric,
            2,
            Some(oid::INT2_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::INT4,
            "int4",
            TypeCategory::Numeric,
            4,
            Some(oid::INT4_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::INT8,
            "int8",
            TypeCategory::Numeric,
            8,
            Some(oid::INT8_ARRAY),
        ));

        // Floats
        self.register(TypeInfo::new(
            oid::FLOAT4,
            "float4",
            TypeCategory::Numeric,
            4,
            Some(oid::FLOAT4_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::FLOAT8,
            "float8",
            TypeCategory::Numeric,
            8,
            Some(oid::FLOAT8_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::NUMERIC,
            "numeric",
            TypeCategory::Numeric,
            -1,
            Some(oid::NUMERIC_ARRAY),
        ));

        // Strings
        self.register(TypeInfo::new(
            oid::TEXT,
            "text",
            TypeCategory::String,
            -1,
            Some(oid::TEXT_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::VARCHAR,
            "varchar",
            TypeCategory::String,
            -1,
            Some(oid::VARCHAR_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::BPCHAR,
            "bpchar",
            TypeCategory::String,
            -1,
            None,
        ));
        self.register(TypeInfo::new(
            oid::CHAR,
            "char",
            TypeCategory::String,
            1,
            Some(oid::CHAR_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::NAME,
            "name",
            TypeCategory::String,
            64,
            Some(oid::NAME_ARRAY),
        ));

        // Binary
        self.register(TypeInfo::new(
            oid::BYTEA,
            "bytea",
            TypeCategory::Binary,
            -1,
            Some(oid::BYTEA_ARRAY),
        ));

        // Date/Time
        self.register(TypeInfo::new(
            oid::DATE,
            "date",
            TypeCategory::DateTime,
            4,
            Some(oid::DATE_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::TIME,
            "time",
            TypeCategory::DateTime,
            8,
            Some(oid::TIME_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::TIMETZ,
            "timetz",
            TypeCategory::DateTime,
            12,
            None,
        ));
        self.register(TypeInfo::new(
            oid::TIMESTAMP,
            "timestamp",
            TypeCategory::DateTime,
            8,
            Some(oid::TIMESTAMP_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::TIMESTAMPTZ,
            "timestamptz",
            TypeCategory::DateTime,
            8,
            Some(oid::TIMESTAMPTZ_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::INTERVAL,
            "interval",
            TypeCategory::DateTime,
            16,
            Some(oid::INTERVAL_ARRAY),
        ));

        // UUID
        self.register(TypeInfo::new(
            oid::UUID,
            "uuid",
            TypeCategory::Uuid,
            16,
            Some(oid::UUID_ARRAY),
        ));

        // JSON
        self.register(TypeInfo::new(
            oid::JSON,
            "json",
            TypeCategory::Json,
            -1,
            Some(oid::JSON_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::JSONB,
            "jsonb",
            TypeCategory::Json,
            -1,
            Some(oid::JSONB_ARRAY),
        ));

        // OID types
        self.register(TypeInfo::new(
            oid::OID,
            "oid",
            TypeCategory::Numeric,
            4,
            Some(oid::OID_ARRAY),
        ));
        self.register(TypeInfo::new(
            oid::XID,
            "xid",
            TypeCategory::Numeric,
            4,
            None,
        ));
        self.register(TypeInfo::new(
            oid::CID,
            "cid",
            TypeCategory::Numeric,
            4,
            None,
        ));

        // Network types
        self.register(TypeInfo::new(
            oid::INET,
            "inet",
            TypeCategory::Network,
            -1,
            None,
        ));
        self.register(TypeInfo::new(
            oid::CIDR,
            "cidr",
            TypeCategory::Network,
            -1,
            None,
        ));
        self.register(TypeInfo::new(
            oid::MACADDR,
            "macaddr",
            TypeCategory::Network,
            6,
            None,
        ));

        // Range types
        self.register(TypeInfo::new(
            oid::INT4RANGE,
            "int4range",
            TypeCategory::Range,
            -1,
            None,
        ));
        self.register(TypeInfo::new(
            oid::INT8RANGE,
            "int8range",
            TypeCategory::Range,
            -1,
            None,
        ));
        self.register(TypeInfo::new(
            oid::NUMRANGE,
            "numrange",
            TypeCategory::Range,
            -1,
            None,
        ));
        self.register(TypeInfo::new(
            oid::TSRANGE,
            "tsrange",
            TypeCategory::Range,
            -1,
            None,
        ));
        self.register(TypeInfo::new(
            oid::TSTZRANGE,
            "tstzrange",
            TypeCategory::Range,
            -1,
            None,
        ));
        self.register(TypeInfo::new(
            oid::DATERANGE,
            "daterange",
            TypeCategory::Range,
            -1,
            None,
        ));

        // Array types
        self.register(TypeInfo::array(oid::BOOL_ARRAY, "bool[]", oid::BOOL));
        self.register(TypeInfo::array(oid::INT2_ARRAY, "int2[]", oid::INT2));
        self.register(TypeInfo::array(oid::INT4_ARRAY, "int4[]", oid::INT4));
        self.register(TypeInfo::array(oid::INT8_ARRAY, "int8[]", oid::INT8));
        self.register(TypeInfo::array(oid::FLOAT4_ARRAY, "float4[]", oid::FLOAT4));
        self.register(TypeInfo::array(oid::FLOAT8_ARRAY, "float8[]", oid::FLOAT8));
        self.register(TypeInfo::array(oid::TEXT_ARRAY, "text[]", oid::TEXT));
        self.register(TypeInfo::array(
            oid::VARCHAR_ARRAY,
            "varchar[]",
            oid::VARCHAR,
        ));
        self.register(TypeInfo::array(oid::BYTEA_ARRAY, "bytea[]", oid::BYTEA));
        self.register(TypeInfo::array(oid::DATE_ARRAY, "date[]", oid::DATE));
        self.register(TypeInfo::array(oid::TIME_ARRAY, "time[]", oid::TIME));
        self.register(TypeInfo::array(
            oid::TIMESTAMP_ARRAY,
            "timestamp[]",
            oid::TIMESTAMP,
        ));
        self.register(TypeInfo::array(
            oid::TIMESTAMPTZ_ARRAY,
            "timestamptz[]",
            oid::TIMESTAMPTZ,
        ));
        self.register(TypeInfo::array(
            oid::INTERVAL_ARRAY,
            "interval[]",
            oid::INTERVAL,
        ));
        self.register(TypeInfo::array(
            oid::NUMERIC_ARRAY,
            "numeric[]",
            oid::NUMERIC,
        ));
        self.register(TypeInfo::array(oid::UUID_ARRAY, "uuid[]", oid::UUID));
        self.register(TypeInfo::array(oid::JSON_ARRAY, "json[]", oid::JSON));
        self.register(TypeInfo::array(oid::JSONB_ARRAY, "jsonb[]", oid::JSONB));
        self.register(TypeInfo::array(oid::OID_ARRAY, "oid[]", oid::OID));
        self.register(TypeInfo::array(oid::CHAR_ARRAY, "char[]", oid::CHAR));
        self.register(TypeInfo::array(oid::NAME_ARRAY, "name[]", oid::NAME));

        // Special types
        self.register(TypeInfo::new(
            oid::UNKNOWN,
            "unknown",
            TypeCategory::Unknown,
            -2,
            None,
        ));
        self.register(TypeInfo::new(
            oid::VOID,
            "void",
            TypeCategory::Unknown,
            4,
            None,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_registry_creation() {
        let registry = TypeRegistry::new();

        // Check basic types exist
        assert!(registry.get(oid::BOOL).is_some());
        assert!(registry.get(oid::INT4).is_some());
        assert!(registry.get(oid::TEXT).is_some());

        // Check by name
        assert!(registry.by_name("int4").is_some());
        assert!(registry.by_name("text").is_some());
    }

    #[test]
    fn test_type_categories() {
        let registry = TypeRegistry::new();

        assert_eq!(registry.category(oid::BOOL), TypeCategory::Boolean);
        assert_eq!(registry.category(oid::INT4), TypeCategory::Numeric);
        assert_eq!(registry.category(oid::TEXT), TypeCategory::String);
        assert_eq!(registry.category(oid::DATE), TypeCategory::DateTime);
        assert_eq!(registry.category(oid::BYTEA), TypeCategory::Binary);
        assert_eq!(registry.category(oid::JSON), TypeCategory::Json);
        assert_eq!(registry.category(oid::UUID), TypeCategory::Uuid);
        assert_eq!(registry.category(oid::INT4_ARRAY), TypeCategory::Array);
    }

    #[test]
    fn test_array_types() {
        let registry = TypeRegistry::new();

        let int4 = registry.get(oid::INT4).unwrap();
        assert_eq!(int4.array_oid, Some(oid::INT4_ARRAY));

        let int4_array = registry.get(oid::INT4_ARRAY).unwrap();
        assert_eq!(int4_array.element_oid, Some(oid::INT4));
    }

    #[test]
    fn test_binary_format_support() {
        let registry = TypeRegistry::new();

        assert!(registry.supports_binary(oid::INT4));
        assert!(registry.supports_binary(oid::TEXT));
        assert!(registry.supports_binary(oid::BOOL));
    }

    #[test]
    fn test_unknown_type() {
        let registry = TypeRegistry::new();

        assert_eq!(registry.category(999_999), TypeCategory::Unknown);
        assert!(!registry.supports_binary(999_999));
    }
}
