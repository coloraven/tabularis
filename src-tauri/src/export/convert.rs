//! Conversions into / out of [`TypedValue`] without DECIMAL→f64.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;

use super::types::{ColumnExportMeta, ExportKind, TypedValue};

/// Default DECIMAL precision when the engine does not advertise (p,s).
pub const DEFAULT_DECIMAL_PRECISION: u8 = 38;
/// Default DECIMAL scale when unknown.
pub const DEFAULT_DECIMAL_SCALE: i8 = 10;

/// Maps a dialect type name (uppercased or mixed) to an [`ExportKind`].
pub fn export_kind_from_type_name(type_name: &str) -> ExportKind {
    let t = type_name.trim().to_ascii_uppercase();
    let base = t
        .split('(')
        .next()
        .unwrap_or(&t)
        .trim()
        .split('[')
        .next()
        .unwrap_or(&t)
        .trim();

    match base {
        "BOOL" | "BOOLEAN" | "BIT" => ExportKind::Boolean,
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "INTEGER" | "BIGINT" | "INT2" | "INT4"
        | "INT8" | "SERIAL" | "BIGSERIAL" | "SMALLSERIAL" | "OID" | "YEAR" => ExportKind::Int64,
        "DECIMAL" | "NUMERIC" | "NEWDECIMAL" | "NUMBER" | "MONEY" => ExportKind::Decimal,
        "FLOAT" | "FLOAT4" | "FLOAT8" | "DOUBLE" | "DOUBLE PRECISION" | "REAL" => {
            ExportKind::Float64
        }
        "DATE" => ExportKind::Date,
        "TIME" | "TIMETZ" => ExportKind::Time,
        "DATETIME" | "TIMESTAMP" | "TIMESTAMP WITHOUT TIME ZONE" => ExportKind::Timestamp,
        "TIMESTAMPTZ" | "TIMESTAMP WITH TIME ZONE" => ExportKind::TimestampTz,
        "JSON" | "JSONB" => ExportKind::Json,
        "BYTEA"
        | "BLOB"
        | "TINYBLOB"
        | "MEDIUMBLOB"
        | "LONGBLOB"
        | "BINARY"
        | "VARBINARY" => ExportKind::Bytes,
        "GEOMETRY"
        | "POINT"
        | "LINESTRING"
        | "POLYGON"
        | "MULTIPOINT"
        | "MULTILINESTRING"
        | "MULTIPOLYGON"
        | "GEOMETRYCOLLECTION"
        | "GEOGRAPHY" => ExportKind::Geometry,
        _ if base.contains("BLOB") || base.contains("BINARY") || base == "BYTEA" => {
            ExportKind::Bytes
        }
        _ if is_geometry_type_name(base) => ExportKind::Geometry,
        _ => ExportKind::Utf8,
    }
}

fn is_geometry_type_name(base: &str) -> bool {
    base.contains("POINT")
        || base.contains("LINESTRING")
        || base.contains("POLYGON")
        || base.contains("COLLECTION")
        || base == "GEOMETRY"
        || base == "GEOGRAPHY"
}

/// Parses optional `TYPE(p,s)` / `TYPE(p)` precision and scale.
pub fn parse_precision_scale(type_name: &str) -> (Option<u8>, Option<i8>) {
    let Some(start) = type_name.find('(') else {
        return (None, None);
    };
    let Some(end) = type_name[start + 1..].find(')') else {
        return (None, None);
    };
    let inner = &type_name[start + 1..start + 1 + end];
    let mut parts = inner.split(',');
    let precision = parts
        .next()
        .and_then(|p| p.trim().parse::<u8>().ok())
        .map(|p| p.min(38));
    let scale = parts
        .next()
        .and_then(|s| s.trim().parse::<i8>().ok())
        .map(|s| s.clamp(0, 38));
    (precision, scale)
}

pub fn column_meta_from_type_name(name: impl Into<String>, type_name: &str) -> ColumnExportMeta {
    let kind = export_kind_from_type_name(type_name);
    let (precision, scale) = parse_precision_scale(type_name);
    let mut meta = ColumnExportMeta::new(name, kind);
    if kind == ExportKind::Decimal {
        meta.precision = precision.or(Some(DEFAULT_DECIMAL_PRECISION));
        meta.scale = scale.or(Some(DEFAULT_DECIMAL_SCALE));
    }
    meta
}

/// Parses a DECIMAL without ever going through `f64`.
pub fn parse_decimal_str(s: &str) -> Result<Decimal, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("empty decimal string".into());
    }
    Decimal::from_str_exact(trimmed)
        .or_else(|_| Decimal::from_str_exact(&trimmed.replace('_', "")))
        .map_err(|e| format!("invalid decimal '{trimmed}': {e}"))
}

/// Rescales `d` to `target_scale` and returns the Arrow Decimal128 coefficient.
///
/// Uses `rust_decimal` only — never `parse::<f64>()`.
pub fn decimal_to_i128(d: &Decimal, target_scale: i8) -> Result<i128, String> {
    let scale = target_scale.clamp(0, 38) as u32;
    let mut value = *d;
    value.rescale(scale);
    Ok(value.mantissa())
}

/// Projects a [`TypedValue`] back to JSON for CSV/JSON/Markdown adapters.
pub fn typed_value_to_json(value: &TypedValue) -> Value {
    match value {
        TypedValue::Null => Value::Null,
        TypedValue::Bool(b) => Value::Bool(*b),
        TypedValue::Int64(i) => Value::from(*i),
        TypedValue::Decimal(d) => Value::String(d.normalize().to_string()),
        TypedValue::Float64(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        TypedValue::Date(d) => Value::String(d.format("%Y-%m-%d").to_string()),
        TypedValue::Time(t) => Value::String(t.format("%H:%M:%S%.f").to_string()),
        TypedValue::Timestamp(ts) => Value::String(ts.format("%Y-%m-%d %H:%M:%S%.f").to_string()),
        TypedValue::TimestampTz(ts) => {
            Value::String(ts.format("%Y-%m-%d %H:%M:%S%.f%:z").to_string())
        }
        TypedValue::Utf8(s) => Value::String(s.clone()),
        TypedValue::Bytes(b) => {
            Value::String(crate::drivers::common::encode_blob_full(b))
        }
        TypedValue::Json(s) => serde_json::from_str(s).unwrap_or_else(|_| Value::String(s.clone())),
        TypedValue::Geometry { wkb, srid } => {
            let hex: String = wkb.iter().map(|byte| format!("{byte:02X}")).collect();
            match srid {
                Some(srid) => Value::String(format!("SRID={srid};0x{hex}")),
                None => Value::String(format!("0x{hex}")),
            }
        }
    }
}

/// Weak JSON → [`TypedValue`] for plugin export when only cell JSON is available.
///
/// When `kind` is known, coerce accordingly. DECIMAL strings never use `f64`.
pub fn typed_value_from_json(value: &Value, kind: ExportKind) -> Result<TypedValue, String> {
    if value.is_null() {
        return Ok(TypedValue::Null);
    }

    match kind {
        ExportKind::Boolean => match value {
            Value::Bool(b) => Ok(TypedValue::Bool(*b)),
            Value::Number(n) => Ok(TypedValue::Bool(n.as_i64().unwrap_or(0) != 0)),
            Value::String(s) => {
                let lower = s.trim().to_ascii_lowercase();
                Ok(TypedValue::Bool(matches!(
                    lower.as_str(),
                    "1" | "true" | "t" | "yes" | "y"
                )))
            }
            other => Err(format!("cannot coerce {other} to boolean")),
        },
        ExportKind::Int64 => match value {
            Value::Number(n) => n
                .as_i64()
                .map(TypedValue::Int64)
                .or_else(|| {
                    n.as_u64()
                        .and_then(|u| i64::try_from(u).ok())
                        .map(TypedValue::Int64)
                })
                .ok_or_else(|| format!("integer out of i64 range: {n}")),
            Value::String(s) => s
                .trim()
                .parse::<i64>()
                .map(TypedValue::Int64)
                .map_err(|e| e.to_string()),
            other => Err(format!("cannot coerce {other} to int64")),
        },
        ExportKind::Decimal => match value {
            Value::String(s) => Ok(TypedValue::Decimal(parse_decimal_str(s)?)),
            Value::Number(n) => {
                // Prefer integer/decimal string forms; reject pure floats as DECIMAL source.
                if let Some(i) = n.as_i64() {
                    return Ok(TypedValue::Decimal(Decimal::from(i)));
                }
                if let Some(u) = n.as_u64() {
                    return Ok(TypedValue::Decimal(Decimal::from(u)));
                }
                Err(format!(
                    "refusing f64 JSON number for DECIMAL (value={n}); use a decimal string"
                ))
            }
            other => Err(format!("cannot coerce {other} to decimal")),
        },
        ExportKind::Float64 => match value {
            Value::Number(n) => n
                .as_f64()
                .map(TypedValue::Float64)
                .ok_or_else(|| format!("invalid float: {n}")),
            Value::String(s) => s
                .trim()
                .parse::<f64>()
                .map(TypedValue::Float64)
                .map_err(|e| e.to_string()),
            other => Err(format!("cannot coerce {other} to float64")),
        },
        ExportKind::Date => match value {
            Value::String(s) => parse_date(s).map(TypedValue::Date),
            other => Err(format!("cannot coerce {other} to date")),
        },
        ExportKind::Time => match value {
            Value::String(s) => parse_time(s).map(TypedValue::Time),
            other => Err(format!("cannot coerce {other} to time")),
        },
        ExportKind::Timestamp => match value {
            Value::String(s) => parse_timestamp(s).map(TypedValue::Timestamp),
            other => Err(format!("cannot coerce {other} to timestamp")),
        },
        ExportKind::TimestampTz => match value {
            Value::String(s) => parse_timestamptz(s).map(TypedValue::TimestampTz),
            other => Err(format!("cannot coerce {other} to timestamptz")),
        },
        ExportKind::Utf8 => Ok(TypedValue::Utf8(json_to_utf8(value))),
        ExportKind::Bytes => bytes_from_json(value).map(TypedValue::Bytes),
        ExportKind::Json => Ok(TypedValue::Json(compact_json(value))),
        ExportKind::Geometry => geometry_from_json(value),
    }
}

/// Infers a weak [`ExportKind`] from a JSON cell when the plugin omits types.
pub fn infer_kind_from_json(value: &Value) -> ExportKind {
    match value {
        Value::Null => ExportKind::Utf8,
        Value::Bool(_) => ExportKind::Boolean,
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                ExportKind::Int64
            } else {
                ExportKind::Float64
            }
        }
        Value::String(s) => {
            if s.starts_with("BLOB:") || s.starts_with("BLOB_FILE_REF:") {
                ExportKind::Bytes
            } else if s.starts_with("0x") && s.len() > 2 && s[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                ExportKind::Geometry
            } else if parse_decimal_str(s).is_ok()
                && s.contains('.')
                && !s.contains('e')
                && !s.contains('E')
            {
                // Prefer Utf8 for arbitrary strings that happen to look numeric;
                // only treat clear decimal-looking values as Decimal when no type.
                ExportKind::Utf8
            } else {
                ExportKind::Utf8
            }
        }
        Value::Array(_) | Value::Object(_) => ExportKind::Json,
    }
}

fn json_to_utf8(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
}

fn bytes_from_json(value: &Value) -> Result<Vec<u8>, String> {
    match value {
        Value::String(s) => {
            if let Some(data) = crate::drivers::common::decode_blob_wire_format(
                s,
                crate::drivers::common::DEFAULT_MAX_BLOB_SIZE,
            ) {
                return Ok(data);
            }
            if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                return decode_hex(hex);
            }
            Ok(s.as_bytes().to_vec())
        }
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                let b = item
                    .as_u64()
                    .ok_or_else(|| "byte array elements must be numbers".to_string())?;
                out.push(u8::try_from(b).map_err(|_| "byte out of range".to_string())?);
            }
            Ok(out)
        }
        other => Err(format!("cannot coerce {other} to bytes")),
    }
}

fn geometry_from_json(value: &Value) -> Result<TypedValue, String> {
    match value {
        Value::String(s) => {
            let (srid, hex) = if let Some(rest) = s.strip_prefix("SRID=") {
                let (srid_part, hex_part) = rest
                    .split_once(';')
                    .ok_or_else(|| "invalid SRID=…;WKB string".to_string())?;
                let srid: i32 = srid_part
                    .parse()
                    .map_err(|e| format!("invalid SRID: {e}"))?;
                let hex = hex_part
                    .strip_prefix("0x")
                    .or_else(|| hex_part.strip_prefix("0X"))
                    .unwrap_or(hex_part);
                (Some(srid), hex)
            } else {
                let hex = s
                    .strip_prefix("0x")
                    .or_else(|| s.strip_prefix("0X"))
                    .unwrap_or(s.as_str());
                (None, hex)
            };
            Ok(TypedValue::Geometry {
                wkb: decode_hex(hex)?,
                srid,
            })
        }
        _ => bytes_from_json(value).map(|wkb| TypedValue::Geometry { wkb, srid: None }),
    }
}

pub fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    let hex = hex.trim();
    if hex.len() % 2 != 0 {
        return Err("hex string must have even length".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("invalid hex at {i}: {e}"))
        })
        .collect()
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    let s = s.trim();
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(s, "%Y/%m/%d"))
        .map_err(|e| format!("invalid date '{s}': {e}"))
}

fn parse_time(s: &str) -> Result<NaiveTime, String> {
    let s = s.trim();
    NaiveTime::parse_from_str(s, "%H:%M:%S%.f")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .map_err(|e| format!("invalid time '{s}': {e}"))
}

fn parse_timestamp(s: &str) -> Result<NaiveDateTime, String> {
    let s = s.trim();
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|e| format!("invalid timestamp '{s}': {e}"))
}

fn parse_timestamptz(s: &str) -> Result<DateTime<Utc>, String> {
    let s = s.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    let naive = parse_timestamp(s)?;
    Ok(naive.and_utc())
}

/// Splits MySQL geometry wire bytes (4-byte LE SRID + WKB) into parts.
pub fn split_mysql_geometry(raw: &[u8]) -> (Option<i32>, Vec<u8>) {
    if raw.len() >= 4 {
        let srid = i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
        (Some(srid), raw[4..].to_vec())
    } else {
        (None, raw.to_vec())
    }
}

#[cfg(test)]
#[path = "convert_tests.rs"]
mod tests;
