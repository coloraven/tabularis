//! Typed streaming extract for Parquet export (full BLOBs, Faithful DECIMAL).

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use sqlx::{Column, Row, TypeInfo, ValueRef};
use uuid::Uuid;

use crate::export::convert::{
    column_meta_from_type_name, parse_decimal_str, split_mysql_geometry,
};
use crate::export::types::{ColumnExportMeta, ExportKind, TypedValue};

use super::extract::resolve_effective_type;

fn is_geometry_type(effective_type: &str) -> bool {
    effective_type == "GEOMETRY"
        || effective_type.contains("POINT")
        || effective_type.contains("LINESTRING")
        || effective_type.contains("POLYGON")
        || effective_type.contains("COLLECTION")
}

pub fn column_meta_from_row(row: &sqlx::mysql::MySqlRow, index: usize) -> ColumnExportMeta {
    let col = row.columns().get(index);
    let name = col
        .map(|c| c.name().to_string())
        .unwrap_or_else(|| format!("col_{index}"));
    let type_name = col.map(|c| c.type_info().name()).unwrap_or("TEXT");
    column_meta_from_type_name(name, type_name)
}

pub fn extract_typed(
    row: &sqlx::mysql::MySqlRow,
    index: usize,
    meta: &ColumnExportMeta,
) -> TypedValue {
    if let Ok(value_ref) = row.try_get_raw(index) {
        if value_ref.is_null() {
            return TypedValue::Null;
        }
    }

    let col = row.columns().get(index);
    let col_type = col.map(|c| c.type_info().name()).unwrap_or("unknown");
    let effective = resolve_effective_type(col_type, None);

    match meta.kind {
        ExportKind::Boolean => extract_bool(row, index),
        ExportKind::Int64 => extract_int(row, index),
        ExportKind::Decimal => extract_decimal(row, index),
        ExportKind::Float64 => extract_float(row, index),
        ExportKind::Date => extract_date(row, index),
        ExportKind::Time => extract_time(row, index),
        ExportKind::Timestamp | ExportKind::TimestampTz => extract_timestamp(row, index),
        ExportKind::Json => extract_json(row, index),
        ExportKind::Bytes => extract_bytes(row, index),
        ExportKind::Geometry => extract_geometry(row, index, &effective),
        ExportKind::Utf8 => extract_utf8(row, index),
    }
}

fn extract_bool(row: &sqlx::mysql::MySqlRow, index: usize) -> TypedValue {
    if let Ok(v) = row.try_get::<bool, _>(index) {
        return TypedValue::Bool(v);
    }
    if let Ok(v) = row.try_get::<i8, _>(index) {
        return TypedValue::Bool(v != 0);
    }
    if let Ok(v) = row.try_get::<i64, _>(index) {
        return TypedValue::Bool(v != 0);
    }
    TypedValue::Null
}

fn extract_int(row: &sqlx::mysql::MySqlRow, index: usize) -> TypedValue {
    if let Ok(v) = row.try_get::<i64, _>(index) {
        return TypedValue::Int64(v);
    }
    if let Ok(v) = row.try_get::<i32, _>(index) {
        return TypedValue::Int64(i64::from(v));
    }
    if let Ok(v) = row.try_get::<i16, _>(index) {
        return TypedValue::Int64(i64::from(v));
    }
    if let Ok(v) = row.try_get::<i8, _>(index) {
        return TypedValue::Int64(i64::from(v));
    }
    if let Ok(v) = row.try_get::<u32, _>(index) {
        return TypedValue::Int64(i64::from(v));
    }
    if let Ok(v) = row.try_get::<u16, _>(index) {
        return TypedValue::Int64(i64::from(v));
    }
    if let Ok(v) = row.try_get::<u8, _>(index) {
        return TypedValue::Int64(i64::from(v));
    }
    if let Ok(v) = row.try_get::<u64, _>(index) {
        return match i64::try_from(v) {
            Ok(i) => TypedValue::Int64(i),
            Err(_) => TypedValue::Utf8(v.to_string()),
        };
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        if let Ok(i) = v.parse::<i64>() {
            return TypedValue::Int64(i);
        }
        return TypedValue::Utf8(v);
    }
    TypedValue::Null
}

fn extract_decimal(row: &sqlx::mysql::MySqlRow, index: usize) -> TypedValue {
    if let Ok(v) = row.try_get::<rust_decimal::Decimal, _>(index) {
        return TypedValue::Decimal(v);
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        return match parse_decimal_str(&v) {
            Ok(d) => TypedValue::Decimal(d),
            Err(_) => TypedValue::Utf8(v),
        };
    }
    TypedValue::Null
}

fn extract_float(row: &sqlx::mysql::MySqlRow, index: usize) -> TypedValue {
    if let Ok(v) = row.try_get::<f64, _>(index) {
        return TypedValue::Float64(v);
    }
    if let Ok(v) = row.try_get::<f32, _>(index) {
        return TypedValue::Float64(f64::from(v));
    }
    TypedValue::Null
}

fn extract_date(row: &sqlx::mysql::MySqlRow, index: usize) -> TypedValue {
    if let Ok(v) = row.try_get::<NaiveDate, _>(index) {
        return TypedValue::Date(v);
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        if let Ok(d) = NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d") {
            return TypedValue::Date(d);
        }
        return TypedValue::Utf8(v);
    }
    TypedValue::Null
}

fn extract_time(row: &sqlx::mysql::MySqlRow, index: usize) -> TypedValue {
    if let Ok(v) = row.try_get::<NaiveTime, _>(index) {
        return TypedValue::Time(v);
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        if let Ok(t) = NaiveTime::parse_from_str(v.trim(), "%H:%M:%S%.f")
            .or_else(|_| NaiveTime::parse_from_str(v.trim(), "%H:%M:%S"))
        {
            return TypedValue::Time(t);
        }
        return TypedValue::Utf8(v);
    }
    TypedValue::Null
}

fn extract_timestamp(row: &sqlx::mysql::MySqlRow, index: usize) -> TypedValue {
    if let Ok(v) = row.try_get::<NaiveDateTime, _>(index) {
        return TypedValue::Timestamp(v);
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        if let Ok(ts) = NaiveDateTime::parse_from_str(v.trim(), "%Y-%m-%d %H:%M:%S%.f")
            .or_else(|_| NaiveDateTime::parse_from_str(v.trim(), "%Y-%m-%d %H:%M:%S"))
        {
            return TypedValue::Timestamp(ts);
        }
        return TypedValue::Utf8(v);
    }
    TypedValue::Null
}

fn extract_json(row: &sqlx::mysql::MySqlRow, index: usize) -> TypedValue {
    if let Ok(v) = row.try_get::<serde_json::Value, _>(index) {
        return TypedValue::Json(serde_json::to_string(&v).unwrap_or_else(|_| "null".into()));
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        return TypedValue::Json(v);
    }
    if let Ok(bytes) = row.try_get::<Vec<u8>, _>(index) {
        if let Ok(text) = String::from_utf8(bytes) {
            return TypedValue::Json(text);
        }
    }
    TypedValue::Null
}

fn extract_bytes(row: &sqlx::mysql::MySqlRow, index: usize) -> TypedValue {
    if let Ok(v) = row.try_get::<Vec<u8>, _>(index) {
        return TypedValue::Bytes(v);
    }
    TypedValue::Null
}

fn extract_geometry(
    row: &sqlx::mysql::MySqlRow,
    index: usize,
    effective: &str,
) -> TypedValue {
    if !is_geometry_type(effective) {
        return extract_bytes(row, index);
    }
    if let Ok(raw_value) = row.try_get_raw(index) {
        if !raw_value.is_null() {
            if let Ok(value) = <Vec<u8> as sqlx::Decode<sqlx::MySql>>::decode(raw_value) {
                let (srid, wkb) = split_mysql_geometry(&value);
                return TypedValue::Geometry { wkb, srid };
            }
        }
    }
    TypedValue::Null
}

fn extract_utf8(row: &sqlx::mysql::MySqlRow, index: usize) -> TypedValue {
    if let Ok(v) = row.try_get::<String, _>(index) {
        return TypedValue::Utf8(v);
    }
    if let Ok(v) = row.try_get::<Uuid, _>(index) {
        return TypedValue::Utf8(v.to_string());
    }
    if let Ok(v) = row.try_get::<Vec<u8>, _>(index) {
        if let Ok(text) = String::from_utf8(v) {
            return TypedValue::Utf8(text);
        }
    }
    TypedValue::Null
}
