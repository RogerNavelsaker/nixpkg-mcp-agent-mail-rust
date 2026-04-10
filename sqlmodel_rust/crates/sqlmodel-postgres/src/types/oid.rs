//! PostgreSQL type Object IDs (OIDs).
//!
//! PostgreSQL identifies types by numeric OIDs. This module defines
//! the well-known OIDs for built-in types.

/// Boolean type
pub const BOOL: u32 = 16;

/// Byte array (bytea)
pub const BYTEA: u32 = 17;

/// Single character (char)
pub const CHAR: u32 = 18;

/// Name type (internal, 63-byte identifier)
pub const NAME: u32 = 19;

/// 8-byte signed integer (int8/bigint)
pub const INT8: u32 = 20;

/// 2-byte signed integer (int2/smallint)
pub const INT2: u32 = 21;

/// 4-byte signed integer (int4/integer)
pub const INT4: u32 = 23;

/// Variable-length text (text)
pub const TEXT: u32 = 25;

/// Object identifier (oid)
pub const OID: u32 = 26;

/// Transaction ID (xid)
pub const XID: u32 = 28;

/// Command ID (cid)
pub const CID: u32 = 29;

/// JSON (text-based)
pub const JSON: u32 = 114;

/// XML data
pub const XML: u32 = 142;

/// Single-precision floating point (float4/real)
pub const FLOAT4: u32 = 700;

/// Double-precision floating point (float8/double precision)
pub const FLOAT8: u32 = 701;

/// Money type
pub const MONEY: u32 = 790;

/// MAC address (6 bytes)
pub const MACADDR: u32 = 829;

/// IPv4/IPv6 CIDR address
pub const CIDR: u32 = 650;

/// IPv4/IPv6 host address
pub const INET: u32 = 869;

/// Variable-length character with limit (varchar)
pub const VARCHAR: u32 = 1043;

/// Fixed-length character (bpchar)
pub const BPCHAR: u32 = 1042;

/// Date (no time)
pub const DATE: u32 = 1082;

/// Time without time zone
pub const TIME: u32 = 1083;

/// Timestamp without time zone
pub const TIMESTAMP: u32 = 1114;

/// Timestamp with time zone
pub const TIMESTAMPTZ: u32 = 1184;

/// Time interval
pub const INTERVAL: u32 = 1186;

/// Time with time zone
pub const TIMETZ: u32 = 1266;

/// Bit string (fixed-length)
pub const BIT: u32 = 1560;

/// Bit string (variable-length)
pub const VARBIT: u32 = 1562;

/// Arbitrary precision numeric
pub const NUMERIC: u32 = 1700;

/// UUID (16-byte identifier)
pub const UUID: u32 = 2950;

/// JSONB (binary JSON)
pub const JSONB: u32 = 3802;

/// Integer range
pub const INT4RANGE: u32 = 3904;

/// Bigint range
pub const INT8RANGE: u32 = 3926;

/// Numeric range
pub const NUMRANGE: u32 = 3906;

/// Timestamp range
pub const TSRANGE: u32 = 3908;

/// Timestamp with time zone range
pub const TSTZRANGE: u32 = 3910;

/// Date range
pub const DATERANGE: u32 = 3912;

// ==================== Array Types ====================
// PostgreSQL array types have their own OIDs

/// Boolean array
pub const BOOL_ARRAY: u32 = 1000;

/// Bytea array
pub const BYTEA_ARRAY: u32 = 1001;

/// Char array
pub const CHAR_ARRAY: u32 = 1002;

/// Name array
pub const NAME_ARRAY: u32 = 1003;

/// Int2 array
pub const INT2_ARRAY: u32 = 1005;

/// Int4 array
pub const INT4_ARRAY: u32 = 1007;

/// Text array
pub const TEXT_ARRAY: u32 = 1009;

/// Varchar array
pub const VARCHAR_ARRAY: u32 = 1015;

/// Int8 array
pub const INT8_ARRAY: u32 = 1016;

/// Float4 array
pub const FLOAT4_ARRAY: u32 = 1021;

/// Float8 array
pub const FLOAT8_ARRAY: u32 = 1022;

/// OID array
pub const OID_ARRAY: u32 = 1028;

/// Timestamp array
pub const TIMESTAMP_ARRAY: u32 = 1115;

/// Date array
pub const DATE_ARRAY: u32 = 1182;

/// Time array
pub const TIME_ARRAY: u32 = 1183;

/// Timestamptz array
pub const TIMESTAMPTZ_ARRAY: u32 = 1185;

/// Interval array
pub const INTERVAL_ARRAY: u32 = 1187;

/// Numeric array
pub const NUMERIC_ARRAY: u32 = 1231;

/// UUID array
pub const UUID_ARRAY: u32 = 2951;

/// JSON array
pub const JSON_ARRAY: u32 = 199;

/// JSONB array
pub const JSONB_ARRAY: u32 = 3807;

// ==================== Special Types ====================

/// Unknown type (used for NULL)
pub const UNKNOWN: u32 = 705;

/// Void type (no return value)
pub const VOID: u32 = 2278;

/// Get the array element OID for an array type OID.
///
/// Returns `None` if the OID is not a known array type.
#[must_use]
pub const fn element_oid(array_oid: u32) -> Option<u32> {
    match array_oid {
        BOOL_ARRAY => Some(BOOL),
        BYTEA_ARRAY => Some(BYTEA),
        CHAR_ARRAY => Some(CHAR),
        NAME_ARRAY => Some(NAME),
        INT2_ARRAY => Some(INT2),
        INT4_ARRAY => Some(INT4),
        TEXT_ARRAY => Some(TEXT),
        VARCHAR_ARRAY => Some(VARCHAR),
        INT8_ARRAY => Some(INT8),
        FLOAT4_ARRAY => Some(FLOAT4),
        FLOAT8_ARRAY => Some(FLOAT8),
        OID_ARRAY => Some(OID),
        TIMESTAMP_ARRAY => Some(TIMESTAMP),
        DATE_ARRAY => Some(DATE),
        TIME_ARRAY => Some(TIME),
        TIMESTAMPTZ_ARRAY => Some(TIMESTAMPTZ),
        INTERVAL_ARRAY => Some(INTERVAL),
        NUMERIC_ARRAY => Some(NUMERIC),
        UUID_ARRAY => Some(UUID),
        JSON_ARRAY => Some(JSON),
        JSONB_ARRAY => Some(JSONB),
        _ => None,
    }
}

/// Get the array type OID for an element type OID.
///
/// Returns `None` if the OID doesn't have a known array type.
#[must_use]
pub const fn array_oid(element_oid: u32) -> Option<u32> {
    match element_oid {
        BOOL => Some(BOOL_ARRAY),
        BYTEA => Some(BYTEA_ARRAY),
        CHAR => Some(CHAR_ARRAY),
        NAME => Some(NAME_ARRAY),
        INT2 => Some(INT2_ARRAY),
        INT4 => Some(INT4_ARRAY),
        TEXT => Some(TEXT_ARRAY),
        VARCHAR => Some(VARCHAR_ARRAY),
        INT8 => Some(INT8_ARRAY),
        FLOAT4 => Some(FLOAT4_ARRAY),
        FLOAT8 => Some(FLOAT8_ARRAY),
        OID => Some(OID_ARRAY),
        TIMESTAMP => Some(TIMESTAMP_ARRAY),
        DATE => Some(DATE_ARRAY),
        TIME => Some(TIME_ARRAY),
        TIMESTAMPTZ => Some(TIMESTAMPTZ_ARRAY),
        INTERVAL => Some(INTERVAL_ARRAY),
        NUMERIC => Some(NUMERIC_ARRAY),
        UUID => Some(UUID_ARRAY),
        JSON => Some(JSON_ARRAY),
        JSONB => Some(JSONB_ARRAY),
        _ => None,
    }
}

/// Check if the OID represents an array type.
#[must_use]
pub const fn is_array(type_oid: u32) -> bool {
    element_oid(type_oid).is_some()
}

/// Get a human-readable name for a type OID.
#[must_use]
pub const fn type_name(type_oid: u32) -> &'static str {
    match type_oid {
        BOOL => "bool",
        BYTEA => "bytea",
        CHAR => "char",
        NAME => "name",
        INT8 => "int8",
        INT2 => "int2",
        INT4 => "int4",
        TEXT => "text",
        OID => "oid",
        XID => "xid",
        CID => "cid",
        JSON => "json",
        XML => "xml",
        FLOAT4 => "float4",
        FLOAT8 => "float8",
        MONEY => "money",
        MACADDR => "macaddr",
        CIDR => "cidr",
        INET => "inet",
        VARCHAR => "varchar",
        BPCHAR => "bpchar",
        DATE => "date",
        TIME => "time",
        TIMESTAMP => "timestamp",
        TIMESTAMPTZ => "timestamptz",
        INTERVAL => "interval",
        TIMETZ => "timetz",
        BIT => "bit",
        VARBIT => "varbit",
        NUMERIC => "numeric",
        UUID => "uuid",
        JSONB => "jsonb",
        INT4RANGE => "int4range",
        INT8RANGE => "int8range",
        NUMRANGE => "numrange",
        TSRANGE => "tsrange",
        TSTZRANGE => "tstzrange",
        DATERANGE => "daterange",
        BOOL_ARRAY => "bool[]",
        BYTEA_ARRAY => "bytea[]",
        CHAR_ARRAY => "char[]",
        NAME_ARRAY => "name[]",
        INT2_ARRAY => "int2[]",
        INT4_ARRAY => "int4[]",
        TEXT_ARRAY => "text[]",
        VARCHAR_ARRAY => "varchar[]",
        INT8_ARRAY => "int8[]",
        FLOAT4_ARRAY => "float4[]",
        FLOAT8_ARRAY => "float8[]",
        OID_ARRAY => "oid[]",
        TIMESTAMP_ARRAY => "timestamp[]",
        DATE_ARRAY => "date[]",
        TIME_ARRAY => "time[]",
        TIMESTAMPTZ_ARRAY => "timestamptz[]",
        INTERVAL_ARRAY => "interval[]",
        NUMERIC_ARRAY => "numeric[]",
        UUID_ARRAY => "uuid[]",
        JSON_ARRAY => "json[]",
        JSONB_ARRAY => "jsonb[]",
        UNKNOWN => "unknown",
        VOID => "void",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_oid_mapping() {
        assert_eq!(element_oid(INT4_ARRAY), Some(INT4));
        assert_eq!(element_oid(TEXT_ARRAY), Some(TEXT));
        assert_eq!(element_oid(UUID_ARRAY), Some(UUID));
        assert_eq!(element_oid(INT4), None);
    }

    #[test]
    fn test_array_oid_mapping() {
        assert_eq!(array_oid(INT4), Some(INT4_ARRAY));
        assert_eq!(array_oid(TEXT), Some(TEXT_ARRAY));
        assert_eq!(array_oid(UUID), Some(UUID_ARRAY));
        assert_eq!(array_oid(UNKNOWN), None);
    }

    #[test]
    fn test_is_array() {
        assert!(is_array(INT4_ARRAY));
        assert!(is_array(TEXT_ARRAY));
        assert!(!is_array(INT4));
        assert!(!is_array(TEXT));
    }

    #[test]
    fn test_type_names() {
        assert_eq!(type_name(INT4), "int4");
        assert_eq!(type_name(TEXT), "text");
        assert_eq!(type_name(INT4_ARRAY), "int4[]");
        assert_eq!(type_name(999_999), "unknown");
    }
}
