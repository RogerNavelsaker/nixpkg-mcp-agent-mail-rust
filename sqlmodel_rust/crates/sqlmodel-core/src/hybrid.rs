//! Hybrid properties: values computed in Rust that also have SQL expression equivalents.
//!
//! A hybrid property can be evaluated in Rust (e.g., `user.full_name()`)
//! or translated to a SQL expression for use in queries
//! (e.g., `User::full_name_expr()` → `first_name || ' ' || last_name`).
//!
//! # Example
//!
//! ```ignore
//! #[derive(Model)]
//! struct User {
//!     first_name: String,
//!     last_name: String,
//!
//!     #[sqlmodel(hybrid, sql = "first_name || ' ' || last_name")]
//!     full_name: Hybrid<String>,
//! }
//!
//! // Rust side: user.full_name holds the computed value
//! // SQL side: User::full_name_expr() returns Expr::raw("first_name || ' ' || last_name")
//! //
//! // Query usage:
//! // select!(User).filter(User::full_name_expr().eq("John Doe"))
//! ```

use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::value::Value;

/// A hybrid property wrapper.
///
/// `Hybrid<T>` holds a Rust-computed value of type `T` and is treated as a
/// computed field by the ORM (excluded from INSERT/UPDATE, initialized via
/// `Default` on load). The macro generates a companion `_expr()` method
/// that returns the SQL expression equivalent.
///
/// `Hybrid<T>` dereferences to `T` for ergonomic access.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Hybrid<T> {
    value: T,
}

impl<T> Hybrid<T> {
    /// Create a new hybrid value.
    pub fn new(value: T) -> Self {
        Self { value }
    }

    /// Get the inner value.
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: Default> Default for Hybrid<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
        }
    }
}

impl<T> Deref for Hybrid<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: fmt::Debug> fmt::Debug for Hybrid<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for Hybrid<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<T: Serialize> Serialize for Hybrid<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Hybrid<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Hybrid::new)
    }
}

impl<T: Into<Value>> From<Hybrid<T>> for Value {
    fn from(h: Hybrid<T>) -> Self {
        h.value.into()
    }
}

impl<T: Clone + Into<Value>> From<&Hybrid<T>> for Value {
    fn from(h: &Hybrid<T>) -> Self {
        h.value.clone().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_deref() {
        let h = Hybrid::new("hello".to_string());
        assert_eq!(h.as_str(), "hello");
        assert_eq!(&*h, "hello");
    }

    #[test]
    fn test_hybrid_default() {
        let h: Hybrid<String> = Hybrid::default();
        assert_eq!(&*h, "");
    }

    #[test]
    fn test_hybrid_display() {
        let h = Hybrid::new(42);
        assert_eq!(format!("{}", h), "42");
    }

    #[test]
    fn test_hybrid_into_value() {
        let h = Hybrid::new("test".to_string());
        let v: Value = h.into();
        assert_eq!(v, Value::Text("test".to_string()));
    }

    #[test]
    fn test_hybrid_serde_roundtrip() {
        let h = Hybrid::new("hello".to_string());
        let json = serde_json::to_string(&h).unwrap();
        assert_eq!(json, "\"hello\"");
        let back: Hybrid<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(&*back, "hello");
    }
}
