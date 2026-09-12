//! Streaming Parquet sink (ZSTD, batched RecordBatches).

use std::io::Write;
use std::sync::Arc;

use arrow::array::{
    ArrayRef, BooleanBuilder, Date32Builder, Decimal128Builder, Float64Builder, Int64Builder,
    LargeBinaryBuilder, LargeStringBuilder, Time64MicrosecondBuilder,
    TimestampMicrosecondBuilder,
};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use chrono::Timelike;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;

use super::convert::decimal_to_i128;
use super::schema::arc_schema;
use super::types::{ColumnExportMeta, ExportKind, TypedValue};

pub const DEFAULT_BATCH_ROWS: usize = 1000;

/// Consumer of typed rows that writes Parquet with ZSTD compression.
pub trait TypedRowSink {
    fn begin(&mut self, columns: &[ColumnExportMeta]) -> Result<(), String>;
    fn write_row(&mut self, values: &[TypedValue]) -> Result<(), String>;
    fn finish(&mut self) -> Result<(), String>;
}

pub struct ParquetSink<W: Write + Send> {
    writer: Option<ArrowWriter<W>>,
    schema: Option<Arc<Schema>>,
    columns: Vec<ColumnExportMeta>,
    buffer: Vec<Vec<TypedValue>>,
    batch_rows: usize,
    inner: Option<W>,
}

impl<W: Write + Send> ParquetSink<W> {
    pub fn new(inner: W) -> Self {
        Self::with_batch_rows(inner, DEFAULT_BATCH_ROWS)
    }

    pub fn with_batch_rows(inner: W, batch_rows: usize) -> Self {
        Self {
            writer: None,
            schema: None,
            columns: Vec::new(),
            buffer: Vec::new(),
            batch_rows: batch_rows.max(1),
            inner: Some(inner),
        }
    }

    fn ensure_writer(&mut self) -> Result<(), String> {
        if self.writer.is_some() {
            return Ok(());
        }
        let schema = self
            .schema
            .clone()
            .ok_or_else(|| "ParquetSink::begin was not called".to_string())?;
        let inner = self
            .inner
            .take()
            .ok_or_else(|| "ParquetSink writer already consumed".to_string())?;
        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(Default::default()))
            .build();
        let writer =
            ArrowWriter::try_new(inner, schema, Some(props)).map_err(|e| e.to_string())?;
        self.writer = Some(writer);
        Ok(())
    }

    fn flush_batch(&mut self) -> Result<(), String> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        self.ensure_writer()?;
        let batch = build_record_batch(
            self.schema.as_ref().expect("schema"),
            &self.columns,
            &self.buffer,
        )?;
        self.writer
            .as_mut()
            .expect("writer")
            .write(&batch)
            .map_err(|e| e.to_string())?;
        self.buffer.clear();
        Ok(())
    }
}

impl<W: Write + Send> TypedRowSink for ParquetSink<W> {
    fn begin(&mut self, columns: &[ColumnExportMeta]) -> Result<(), String> {
        if columns.is_empty() {
            return Err("cannot export Parquet with zero columns".into());
        }
        self.columns = columns.to_vec();
        self.schema = Some(arc_schema(columns));
        Ok(())
    }

    fn write_row(&mut self, values: &[TypedValue]) -> Result<(), String> {
        if self.schema.is_none() {
            return Err("ParquetSink::begin was not called".into());
        }
        if values.len() != self.columns.len() {
            return Err(format!(
                "row has {} values but schema has {} columns",
                values.len(),
                self.columns.len()
            ));
        }
        self.buffer.push(values.to_vec());
        if self.buffer.len() >= self.batch_rows {
            self.flush_batch()?;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        // Empty result: still emit a valid Parquet file with schema and no row groups.
        self.ensure_writer()?;
        self.flush_batch()?;
        if let Some(writer) = self.writer.take() {
            writer.close().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

fn build_record_batch(
    schema: &Arc<Schema>,
    columns: &[ColumnExportMeta],
    rows: &[Vec<TypedValue>],
) -> Result<RecordBatch, String> {
    let mut arrays: Vec<ArrayRef> = Vec::with_capacity(columns.len());
    for (col_idx, col) in columns.iter().enumerate() {
        let array = build_column_array(col, rows, col_idx)?;
        arrays.push(array);
    }
    RecordBatch::try_new(schema.clone(), arrays).map_err(|e| e.to_string())
}

fn build_column_array(
    col: &ColumnExportMeta,
    rows: &[Vec<TypedValue>],
    col_idx: usize,
) -> Result<ArrayRef, String> {
    match col.kind {
        ExportKind::Boolean => {
            let mut b = BooleanBuilder::with_capacity(rows.len());
            for row in rows {
                match &row[col_idx] {
                    TypedValue::Null => b.append_null(),
                    TypedValue::Bool(v) => b.append_value(*v),
                    other => {
                        return Err(format!(
                            "column '{}': expected Bool, got {other:?}",
                            col.name
                        ))
                    }
                }
            }
            Ok(Arc::new(b.finish()))
        }
        ExportKind::Int64 => {
            let mut b = Int64Builder::with_capacity(rows.len());
            for row in rows {
                match &row[col_idx] {
                    TypedValue::Null => b.append_null(),
                    TypedValue::Int64(v) => b.append_value(*v),
                    TypedValue::Utf8(s) => {
                        let parsed = s.parse::<i64>().map_err(|_| {
                            format!(
                                "column '{}': value '{s}' does not fit Int64",
                                col.name
                            )
                        })?;
                        b.append_value(parsed);
                    }
                    other => {
                        return Err(format!(
                            "column '{}': expected Int64, got {other:?}",
                            col.name
                        ))
                    }
                }
            }
            Ok(Arc::new(b.finish()))
        }
        ExportKind::Decimal => {
            let precision = col.decimal_precision();
            let scale = col.decimal_scale();
            let mut b = Decimal128Builder::with_capacity(rows.len());
            for row in rows {
                match &row[col_idx] {
                    TypedValue::Null => b.append_null(),
                    TypedValue::Decimal(d) => {
                        let coeff = decimal_to_i128(d, scale)?;
                        b.append_value(coeff);
                    }
                    other => {
                        return Err(format!(
                            "column '{}': expected Decimal, got {other:?}",
                            col.name
                        ))
                    }
                }
            }
            let array = b
                .finish()
                .with_precision_and_scale(precision, scale)
                .map_err(|e| e.to_string())?;
            Ok(Arc::new(array))
        }
        ExportKind::Float64 => {
            let mut b = Float64Builder::with_capacity(rows.len());
            for row in rows {
                match &row[col_idx] {
                    TypedValue::Null => b.append_null(),
                    TypedValue::Float64(v) => b.append_value(*v),
                    other => {
                        return Err(format!(
                            "column '{}': expected Float64, got {other:?}",
                            col.name
                        ))
                    }
                }
            }
            Ok(Arc::new(b.finish()))
        }
        ExportKind::Date => {
            let mut b = Date32Builder::with_capacity(rows.len());
            for row in rows {
                match &row[col_idx] {
                    TypedValue::Null => b.append_null(),
                    TypedValue::Date(d) => b.append_value(date32_days(d)),
                    other => {
                        return Err(format!(
                            "column '{}': expected Date, got {other:?}",
                            col.name
                        ))
                    }
                }
            }
            Ok(Arc::new(b.finish()))
        }
        ExportKind::Time => {
            let mut b = Time64MicrosecondBuilder::with_capacity(rows.len());
            for row in rows {
                match &row[col_idx] {
                    TypedValue::Null => b.append_null(),
                    TypedValue::Time(t) => b.append_value(time_to_micros(t)),
                    other => {
                        return Err(format!(
                            "column '{}': expected Time, got {other:?}",
                            col.name
                        ))
                    }
                }
            }
            Ok(Arc::new(b.finish()))
        }
        ExportKind::Timestamp | ExportKind::TimestampTz => {
            let mut b = TimestampMicrosecondBuilder::with_capacity(rows.len());
            for row in rows {
                match &row[col_idx] {
                    TypedValue::Null => b.append_null(),
                    TypedValue::Timestamp(ts) => b.append_value(naive_ts_to_micros(ts)),
                    TypedValue::TimestampTz(ts) => b.append_value(ts.timestamp_micros()),
                    other => {
                        return Err(format!(
                            "column '{}': expected Timestamp, got {other:?}",
                            col.name
                        ))
                    }
                }
            }
            let array = b.finish();
            if col.kind == ExportKind::TimestampTz {
                let tz = col.tz.as_deref().unwrap_or("UTC");
                return Ok(Arc::new(array.with_timezone(tz)));
            }
            Ok(Arc::new(array))
        }
        ExportKind::Utf8 | ExportKind::Json => {
            let mut b = LargeStringBuilder::with_capacity(rows.len(), rows.len() * 16);
            for row in rows {
                match &row[col_idx] {
                    TypedValue::Null => b.append_null(),
                    TypedValue::Utf8(s) | TypedValue::Json(s) => b.append_value(s),
                    other => {
                        return Err(format!(
                            "column '{}': expected Utf8/Json, got {other:?}",
                            col.name
                        ))
                    }
                }
            }
            Ok(Arc::new(b.finish()))
        }
        ExportKind::Bytes | ExportKind::Geometry => {
            let mut b = LargeBinaryBuilder::with_capacity(rows.len(), rows.len() * 32);
            for row in rows {
                match &row[col_idx] {
                    TypedValue::Null => b.append_null(),
                    TypedValue::Bytes(bytes) => b.append_value(bytes),
                    TypedValue::Geometry { wkb, .. } => b.append_value(wkb),
                    other => {
                        return Err(format!(
                            "column '{}': expected Bytes/Geometry, got {other:?}",
                            col.name
                        ))
                    }
                }
            }
            Ok(Arc::new(b.finish()))
        }
    }
}

fn date32_days(d: &chrono::NaiveDate) -> i32 {
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch");
    (*d - epoch).num_days() as i32
}

fn time_to_micros(t: &chrono::NaiveTime) -> i64 {
    t.num_seconds_from_midnight() as i64 * 1_000_000 + (t.nanosecond() / 1_000) as i64
}

fn naive_ts_to_micros(ts: &chrono::NaiveDateTime) -> i64 {
    ts.and_utc().timestamp_micros()
}

#[cfg(test)]
#[path = "parquet_sink_tests.rs"]
mod tests;
