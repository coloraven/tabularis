use super::*;
use crate::export::types::{ColumnExportMeta, ExportKind};

#[test]
fn numeric_never_becomes_float64() {
    let cols = vec![
        ColumnExportMeta::with_decimal("amount", 10, 2),
        ColumnExportMeta::new("rate", ExportKind::Float64),
    ];
    let schema = arrow_schema_from_columns(&cols);
    assert_eq!(
        schema.field(0).data_type(),
        &DataType::Decimal128(10, 2)
    );
    assert_eq!(schema.field(1).data_type(), &DataType::Float64);
}

#[test]
fn geometry_adds_geoparquet_metadata() {
    let mut geom = ColumnExportMeta::new("g", ExportKind::Geometry);
    geom.srid = Some(4326);
    let schema = arrow_schema_from_columns(&[geom]);
    let geo = schema.metadata().get("geo").expect("geo metadata");
    assert!(geo.contains("\"encoding\":\"WKB\""));
    assert!(geo.contains("EPSG:4326"));
}
