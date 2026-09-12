//! Arrow schema factory for Faithful Parquet export.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, TimeUnit};

use super::types::{ColumnExportMeta, ExportKind};

/// Builds an Arrow schema from export column metadata.
///
/// NUMERIC/DECIMAL always become `Decimal128` — never `Float64`.
pub fn arrow_schema_from_columns(columns: &[ColumnExportMeta]) -> Schema {
    let fields: Vec<Field> = columns.iter().map(field_for_column).collect();
    let mut metadata = std::collections::HashMap::new();

    if let Some(geo_json) = geoparquet_metadata(columns) {
        metadata.insert("geo".to_string(), geo_json);
    }

    Schema::new_with_metadata(fields, metadata)
}

fn field_for_column(col: &ColumnExportMeta) -> Field {
    let data_type = match col.kind {
        ExportKind::Boolean => DataType::Boolean,
        ExportKind::Int64 => DataType::Int64,
        ExportKind::Decimal => {
            DataType::Decimal128(col.decimal_precision(), col.decimal_scale())
        }
        ExportKind::Float64 => DataType::Float64,
        ExportKind::Date => DataType::Date32,
        ExportKind::Time => DataType::Time64(TimeUnit::Microsecond),
        ExportKind::Timestamp => DataType::Timestamp(TimeUnit::Microsecond, None),
        ExportKind::TimestampTz => {
            let tz = col
                .tz
                .clone()
                .unwrap_or_else(|| "UTC".to_string());
            DataType::Timestamp(TimeUnit::Microsecond, Some(tz.into()))
        }
        ExportKind::Utf8 | ExportKind::Json => DataType::LargeUtf8,
        ExportKind::Bytes | ExportKind::Geometry => DataType::LargeBinary,
    };

    let mut field = Field::new(&col.name, data_type, true);
    if col.kind == ExportKind::Geometry {
        let mut meta = std::collections::HashMap::new();
        meta.insert("encoding".to_string(), "WKB".to_string());
        if let Some(srid) = col.srid {
            meta.insert("srid".to_string(), srid.to_string());
        }
        field = field.with_metadata(meta);
    }
    field
}

/// GeoParquet 1.0-style file metadata when at least one geometry column exists.
fn geoparquet_metadata(columns: &[ColumnExportMeta]) -> Option<String> {
    let geo_cols: Vec<&ColumnExportMeta> = columns
        .iter()
        .filter(|c| c.kind == ExportKind::Geometry)
        .collect();
    if geo_cols.is_empty() {
        return None;
    }

    let primary = geo_cols[0].name.clone();
    let mut columns_obj = serde_json::Map::new();
    for col in geo_cols {
        let mut entry = serde_json::Map::new();
        entry.insert("encoding".into(), serde_json::Value::String("WKB".into()));
        entry.insert(
            "geometry_types".into(),
            serde_json::json!(["Geometry"]),
        );
        if let Some(srid) = col.srid {
            entry.insert(
                "crs".into(),
                serde_json::Value::String(format!("EPSG:{srid}")),
            );
        }
        columns_obj.insert(col.name.clone(), serde_json::Value::Object(entry));
    }

    let root = serde_json::json!({
        "version": "1.0.0",
        "primary_column": primary,
        "columns": columns_obj,
    });
    Some(root.to_string())
}

pub fn arc_schema(columns: &[ColumnExportMeta]) -> Arc<Schema> {
    Arc::new(arrow_schema_from_columns(columns))
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
