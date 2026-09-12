use super::*;
use super::convert::parse_decimal_str;
use super::types::{ColumnExportMeta, ExportKind, TypedValue};
use arrow::array::Decimal128Array;
use arrow::datatypes::DataType;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::io::{Seek, SeekFrom, Write};

#[test]
fn parquet_roundtrip_preserves_decimal128() {
    let cols = vec![ColumnExportMeta::with_decimal("amount", 10, 2)];
    let mut file = tempfile::tempfile().expect("tempfile");
    {
        let mut sink = ParquetSink::with_batch_rows(&mut file, 10);
        sink.begin(&cols).unwrap();
        sink.write_row(&[TypedValue::Decimal(parse_decimal_str("99.99").unwrap())])
            .unwrap();
        sink.finish().unwrap();
    }
    file.flush().unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();

    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();
    let batches: Vec<_> = reader.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(batches.len(), 1);
    let batch = &batches[0];
    assert_eq!(batch.num_rows(), 1);
    assert_eq!(
        batch.schema().field(0).data_type(),
        &DataType::Decimal128(10, 2)
    );
    let arr = batch
        .column(0)
        .as_any()
        .downcast_ref::<Decimal128Array>()
        .expect("decimal array");
    assert_eq!(arr.value(0), 9999);
}

#[test]
fn numeric_schema_is_not_float() {
    let cols = vec![ColumnExportMeta::with_decimal("n", 38, 10)];
    let schema = super::super::schema::arrow_schema_from_columns(&cols);
    assert!(matches!(
        schema.field(0).data_type(),
        DataType::Decimal128(38, 10)
    ));
}
