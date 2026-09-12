use super::*;
use serde_json::json;

#[test]
fn decimal_type_maps_to_decimal_not_float() {
    assert_eq!(export_kind_from_type_name("DECIMAL(10,2)"), ExportKind::Decimal);
    assert_eq!(export_kind_from_type_name("NUMERIC"), ExportKind::Decimal);
    assert_eq!(export_kind_from_type_name("NEWDECIMAL"), ExportKind::Decimal);
    assert_ne!(export_kind_from_type_name("DECIMAL"), ExportKind::Float64);
}

#[test]
fn float_types_map_to_float64_only() {
    assert_eq!(export_kind_from_type_name("DOUBLE"), ExportKind::Float64);
    assert_eq!(export_kind_from_type_name("REAL"), ExportKind::Float64);
    assert_eq!(export_kind_from_type_name("FLOAT4"), ExportKind::Float64);
}

#[test]
fn parse_precision_scale_from_type_name() {
    assert_eq!(parse_precision_scale("DECIMAL(10,2)"), (Some(10), Some(2)));
    assert_eq!(parse_precision_scale("NUMERIC(38)"), (Some(38), None));
    assert_eq!(parse_precision_scale("TEXT"), (None, None));
}

#[test]
fn parse_decimal_str_never_needs_f64() {
    let d = parse_decimal_str("12345678901234567890.123456789").unwrap();
    assert_eq!(d.to_string(), "12345678901234567890.123456789");
}

#[test]
fn decimal_to_i128_rescales() {
    let d = parse_decimal_str("12.30").unwrap();
    assert_eq!(decimal_to_i128(&d, 2).unwrap(), 1230);
}

#[test]
fn refuse_f64_json_number_for_decimal() {
    let err = typed_value_from_json(&json!(1.25), ExportKind::Decimal).unwrap_err();
    assert!(err.contains("f64") || err.contains("DECIMAL"));
}

#[test]
fn decimal_from_string_json() {
    let v = typed_value_from_json(&json!("99.99"), ExportKind::Decimal).unwrap();
    match v {
        TypedValue::Decimal(d) => assert_eq!(d.to_string(), "99.99"),
        other => panic!("expected Decimal, got {other:?}"),
    }
}

#[test]
fn geometry_from_hex_json() {
    let v = typed_value_from_json(&json!("0x0101000000000000000000F03F0000000000000040"), ExportKind::Geometry)
        .unwrap();
    match v {
        TypedValue::Geometry { wkb, srid: None } => {
            assert_eq!(wkb.first().copied(), Some(0x01));
        }
        other => panic!("expected Geometry, got {other:?}"),
    }
}

#[test]
fn split_mysql_geometry_strips_srid() {
    let mut raw = vec![0xE6, 0x10, 0x00, 0x00]; // SRID 4326 LE
    raw.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x00]);
    let (srid, wkb) = split_mysql_geometry(&raw);
    assert_eq!(srid, Some(4326));
    assert_eq!(wkb, vec![0x01, 0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn column_meta_decimal_defaults() {
    let meta = column_meta_from_type_name("amount", "DECIMAL");
    assert_eq!(meta.kind, ExportKind::Decimal);
    assert_eq!(meta.decimal_precision(), 38);
    assert_eq!(meta.decimal_scale(), 10);
}
