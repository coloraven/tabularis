//! Typed streaming extract for PostgreSQL Parquet export.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use tokio_postgres::Row;
use uuid::Uuid;

use crate::export::convert::{column_meta_from_type_name, decode_hex, parse_decimal_str};
use crate::export::types::{ColumnExportMeta, ExportKind, TypedValue};

pub fn column_meta_from_row(row: &Row, index: usize) -> ColumnExportMeta {
    let col = &row.columns()[index];
    column_meta_from_type_name(col.name(), col.type_().name())
}

pub fn extract_typed(row: &Row, index: usize, meta: &ColumnExportMeta) -> TypedValue {
    match meta.kind {
        ExportKind::Boolean => get_opt(row, index, TypedValue::Bool),
        ExportKind::Int64 => extract_int(row, index),
        ExportKind::Decimal => extract_decimal(row, index),
        ExportKind::Float64 => {
            if let Ok(Some(v)) = row.try_get::<_, Option<f64>>(index) {
                TypedValue::Float64(v)
            } else if let Ok(Some(v)) = row.try_get::<_, Option<f32>>(index) {
                TypedValue::Float64(f64::from(v))
            } else {
                TypedValue::Null
            }
        }
        ExportKind::Date => get_opt(row, index, TypedValue::Date),
        ExportKind::Time => get_opt(row, index, TypedValue::Time),
        ExportKind::Timestamp => get_opt(row, index, TypedValue::Timestamp),
        ExportKind::TimestampTz => {
            get_opt(row, index, |v: DateTime<Utc>| TypedValue::TimestampTz(v))
        }
        ExportKind::Json => extract_json(row, index),
        ExportKind::Bytes => get_opt(row, index, TypedValue::Bytes),
        ExportKind::Geometry => extract_geometry(row, index),
        ExportKind::Utf8 => extract_utf8(row, index),
    }
}

fn get_opt<T, F>(row: &Row, index: usize, map: F) -> TypedValue
where
    T: for<'a> tokio_postgres::types::FromSql<'a>,
    F: FnOnce(T) -> TypedValue,
{
    match row.try_get::<_, Option<T>>(index) {
        Ok(Some(v)) => map(v),
        Ok(None) | Err(_) => TypedValue::Null,
    }
}

fn extract_int(row: &Row, index: usize) -> TypedValue {
    if let Ok(Some(v)) = row.try_get::<_, Option<i64>>(index) {
        return TypedValue::Int64(v);
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<i32>>(index) {
        return TypedValue::Int64(i64::from(v));
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<i16>>(index) {
        return TypedValue::Int64(i64::from(v));
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<i8>>(index) {
        return TypedValue::Int64(i64::from(v));
    }
    TypedValue::Null
}

fn extract_decimal(row: &Row, index: usize) -> TypedValue {
    if let Ok(Some(v)) = row.try_get::<_, Option<Decimal>>(index) {
        return TypedValue::Decimal(v);
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<String>>(index) {
        return match parse_decimal_str(&v) {
            Ok(d) => TypedValue::Decimal(d),
            Err(_) => TypedValue::Utf8(v),
        };
    }
    TypedValue::Null
}

fn extract_json(row: &Row, index: usize) -> TypedValue {
    if let Ok(Some(v)) = row.try_get::<_, Option<serde_json::Value>>(index) {
        return TypedValue::Json(serde_json::to_string(&v).unwrap_or_else(|_| "null".into()));
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<String>>(index) {
        return TypedValue::Json(v);
    }
    TypedValue::Null
}

fn extract_geometry(row: &Row, index: usize) -> TypedValue {
    if let Ok(Some(v)) = row.try_get::<_, Option<Vec<u8>>>(index) {
        return TypedValue::Geometry { wkb: v, srid: None };
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<String>>(index) {
        if let Some(hex) = v
            .strip_prefix("\\x")
            .or_else(|| v.strip_prefix("0x"))
            .or_else(|| v.strip_prefix("0X"))
        {
            if let Ok(wkb) = decode_hex(hex) {
                return TypedValue::Geometry { wkb, srid: None };
            }
        }
        return TypedValue::Utf8(v);
    }
    TypedValue::Null
}

fn extract_utf8(row: &Row, index: usize) -> TypedValue {
    if let Ok(Some(v)) = row.try_get::<_, Option<String>>(index) {
        return TypedValue::Utf8(v);
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<Uuid>>(index) {
        return TypedValue::Utf8(v.to_string());
    }
    let json = super::extract::extract_value(row, index, None);
    match json {
        serde_json::Value::Null => TypedValue::Null,
        serde_json::Value::String(s) => TypedValue::Utf8(s),
        other => TypedValue::Utf8(other.to_string()),
    }
}
