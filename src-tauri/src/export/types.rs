//! Intermediate export types between driver-native values and Arrow/Parquet.
//!
//! Drivers project dialect-specific columns into [`ExportKind`] + [`TypedValue`]
//! before any sink sees them. DECIMAL never travels through `f64`.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use rust_decimal::Decimal;

/// Logical type used for Arrow schema construction and value conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Boolean,
    Int64,
    /// Fixed-point decimal. Prefer this over Float64 for NUMERIC/DECIMAL/MONEY.
    Decimal,
    /// Only for genuine floating-point columns (FLOAT/DOUBLE/REAL).
    Float64,
    Date,
    Time,
    Timestamp,
    TimestampTz,
    Utf8,
    Bytes,
    Json,
    Geometry,
}

/// Stable column metadata for one export column (schema side).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnExportMeta {
    pub name: String,
    pub kind: ExportKind,
    /// DECIMAL precision (Arrow Decimal128 max 38). `None` → default 38.
    pub precision: Option<u8>,
    /// DECIMAL scale. `None` → default 10 when kind is Decimal.
    pub scale: Option<i8>,
    /// IANA / Postgres timezone name for TimestampTz, when known.
    pub tz: Option<String>,
    /// Spatial reference id for Geometry (e.g. MySQL SRID prefix).
    pub srid: Option<i32>,
}

impl ColumnExportMeta {
    pub fn new(name: impl Into<String>, kind: ExportKind) -> Self {
        Self {
            name: name.into(),
            kind,
            precision: None,
            scale: None,
            tz: None,
            srid: None,
        }
    }

    pub fn with_decimal(name: impl Into<String>, precision: u8, scale: i8) -> Self {
        Self {
            name: name.into(),
            kind: ExportKind::Decimal,
            precision: Some(precision),
            scale: Some(scale),
            tz: None,
            srid: None,
        }
    }

    pub fn decimal_precision(&self) -> u8 {
        self.precision.unwrap_or(38).min(38)
    }

    pub fn decimal_scale(&self) -> i8 {
        self.scale.unwrap_or(10).clamp(0, 38)
    }
}

/// Runtime cell value aligned with [`ExportKind`].
#[derive(Debug, Clone, PartialEq)]
pub enum TypedValue {
    Null,
    Bool(bool),
    Int64(i64),
    Decimal(Decimal),
    Float64(f64),
    Date(NaiveDate),
    Time(NaiveTime),
    Timestamp(NaiveDateTime),
    TimestampTz(DateTime<Utc>),
    Utf8(String),
    Bytes(Vec<u8>),
    /// Canonical JSON text (compact).
    Json(String),
    Geometry {
        wkb: Vec<u8>,
        srid: Option<i32>,
    },
}

impl TypedValue {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}
