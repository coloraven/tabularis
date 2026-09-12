//! Typed streaming extract for SQLite Parquet export.

use rust_decimal::Decimal;
use sqlx::{Column, Row, TypeInfo, ValueRef};

use crate::export::{
    column_meta_from_type_name, parse_decimal_str, ColumnExportMeta, ExportKind, TypedValue,
};

pub fn column_meta_from_row(row: &sqlx::sqlite::SqliteRow, index: usize) -> ColumnExportMeta {
    let col = row.columns().get(index);
    let name = col
        .map(|c| c.name().to_string())
        .unwrap_or_else(|| format!("col_{index}"));
    // Declared type if present; otherwise infer from affinity name.
    let type_name = col
        .map(|c| c.type_info().name())
        .unwrap_or("TEXT");
    let mut meta = column_meta_from_type_name(name, type_name);
    // SQLite NUMERIC affinity often surfaces as "NUMERIC" / "DECIMAL".
    if type_name.eq_ignore_ascii_case("NUMERIC") || type_name.eq_ignore_ascii_case("DECIMAL") {
        meta.kind = ExportKind::Decimal;
    }
    meta
}

pub fn extract_typed(
    row: &sqlx::sqlite::SqliteRow,
    index: usize,
    meta: &ColumnExportMeta,
) -> TypedValue {
    if let Ok(val_ref) = row.try_get_raw(index) {
        if val_ref.is_null() {
            return TypedValue::Null;
        }
    }

    match meta.kind {
        ExportKind::Boolean => {
            if let Ok(v) = row.try_get::<i64, _>(index) {
                return TypedValue::Bool(v != 0);
            }
            TypedValue::Null
        }
        ExportKind::Int64 => {
            if let Ok(v) = row.try_get::<i64, _>(index) {
                return TypedValue::Int64(v);
            }
            TypedValue::Null
        }
        ExportKind::Decimal => {
            if let Ok(v) = row.try_get::<String, _>(index) {
                return match parse_decimal_str(&v) {
                    Ok(d) => TypedValue::Decimal(d),
                    Err(_) => TypedValue::Utf8(v),
                };
            }
            if let Ok(v) = row.try_get::<i64, _>(index) {
                return TypedValue::Decimal(Decimal::from(v));
            }
            // No f64 path for DECIMAL columns.
            TypedValue::Null
        }
        ExportKind::Float64 => {
            if let Ok(v) = row.try_get::<f64, _>(index) {
                return TypedValue::Float64(v);
            }
            TypedValue::Null
        }
        ExportKind::Bytes => {
            if let Ok(v) = row.try_get::<Vec<u8>, _>(index) {
                return TypedValue::Bytes(v);
            }
            TypedValue::Null
        }
        ExportKind::Json => {
            if let Ok(v) = row.try_get::<String, _>(index) {
                return TypedValue::Json(v);
            }
            TypedValue::Null
        }
        _ => {
            if let Ok(v) = row.try_get::<String, _>(index) {
                return TypedValue::Utf8(v);
            }
            if let Ok(v) = row.try_get::<i64, _>(index) {
                return TypedValue::Int64(v);
            }
            if let Ok(v) = row.try_get::<f64, _>(index) {
                return TypedValue::Float64(v);
            }
            if let Ok(v) = row.try_get::<Vec<u8>, _>(index) {
                return TypedValue::Bytes(v);
            }
            TypedValue::Null
        }
    }
}
