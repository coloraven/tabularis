mod convert;
mod format;
mod parquet_sink;
mod progress;
mod schema;
mod sink;
mod types;

#[cfg(test)]
mod tests;

pub use format::{parse_csv_delimiter, value_to_csv_string, ExportFormat, DEFAULT_CSV_DELIMITER};
pub use parquet_sink::{ParquetSink, TypedRowSink, DEFAULT_BATCH_ROWS};
pub use progress::{ProgressEmitter, DEFAULT_INTERVAL as DEFAULT_PROGRESS_INTERVAL};
pub use sink::{CsvSink, JsonSink, MarkdownSink, RowSink};
pub use types::{ColumnExportMeta, ExportKind, TypedValue};
pub use convert::{
    column_meta_from_type_name, decode_hex, parse_decimal_str, split_mysql_geometry,
};

use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime, State};

use crate::commands::{
    expand_k8s_connection_params, expand_ssh_connection_params, find_connection_by_id,
    register_abort_handle, resolve_connection_params_with_id, unregister_abort_handle,
    AbortHandleMap,
};
use crate::drivers::{mysql, postgres, sqlite};
use crate::models::ConnectionParams;

use convert::{column_meta_from_type_name, infer_kind_from_json, typed_value_from_json};
use types::{ColumnExportMeta as ColMeta, TypedValue as TVal};

pub struct ExportCancellationState {
    pub handles: Arc<Mutex<AbortHandleMap>>,
}

impl Default for ExportCancellationState {
    fn default() -> Self {
        Self {
            handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[derive(Clone, Serialize)]
struct ExportProgressPayload {
    rows_processed: u64,
}

const EXPORT_PROGRESS_EVENT: &str = "export_progress";

fn sanitize_query(query: &str) -> String {
    query.trim().trim_end_matches(';').to_string()
}

#[tauri::command]
pub async fn cancel_export(
    state: State<'_, ExportCancellationState>,
    connection_id: String,
) -> Result<(), String> {
    let entries = {
        let mut handles = state.handles.lock().unwrap();
        handles.remove(&connection_id).unwrap_or_default()
    };
    for handle in entries {
        handle.abort();
    }
    Ok(())
}

#[tauri::command]
pub async fn export_query_to_file<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ExportCancellationState>,
    connection_id: String,
    query: String,
    file_path: String,
    format: String,
    csv_delimiter: Option<String>,
    database: Option<String>,
) -> Result<(), String> {
    let sanitized_query = sanitize_query(&query);
    let saved_conn = find_connection_by_id(&app, &connection_id)?;
    let expanded_params = expand_ssh_connection_params(&app, &saved_conn.params).await?;
    let expanded_params = expand_k8s_connection_params(&app, &expanded_params).await?;
    let mut params = resolve_connection_params_with_id(&expanded_params, &connection_id)?;
    // Scope the export to the selected database on connections that expose multiple
    // databases (e.g. MySQL/MariaDB), so the query runs against the database the
    // user is viewing rather than the connection's primary database.
    if let Some(db) = database {
        params.database = crate::models::DatabaseSelection::Single(db);
    }
    let driver = saved_conn.params.driver.clone();

    let export_format = ExportFormat::parse(&format)?;
    let delimiter = parse_csv_delimiter(csv_delimiter.as_deref());

    let app_for_task = app.clone();
    let task_connection_id = connection_id.clone();

    let task = tokio::spawn(async move {
        let file = File::create(&file_path).map_err(|e| e.to_string())?;
        let writer = BufWriter::new(file);
        run_export(
            app_for_task,
            &driver,
            &params,
            &sanitized_query,
            writer,
            export_format,
            delimiter,
        )
        .await
    });

    let abort_handle = Arc::new(task.abort_handle());
    register_abort_handle(
        &state.handles,
        task_connection_id.clone(),
        abort_handle.clone(),
    );

    let result = task.await;

    unregister_abort_handle(&state.handles, &task_connection_id, &abort_handle);

    match result {
        Ok(res) => res,
        Err(_) => Err("Export cancelled".into()),
    }
}

/// Wires the driver stream, the row sink, and the progress emitter together.
/// Kept as a free function so the spawned task body stays linear and the
/// pieces remain individually unit-testable.
async fn run_export<R: Runtime>(
    app: AppHandle<R>,
    driver: &str,
    params: &ConnectionParams,
    query: &str,
    writer: BufWriter<File>,
    format: ExportFormat,
    delimiter: u8,
) -> Result<(), String> {
    let app_for_progress = app.clone();
    let mut progress = ProgressEmitter::new(DEFAULT_PROGRESS_INTERVAL, move |count| {
        let _ = app_for_progress.emit(
            EXPORT_PROGRESS_EVENT,
            ExportProgressPayload {
                rows_processed: count,
            },
        );
    });

    match format {
        ExportFormat::Csv => {
            let mut sink = CsvSink::new(writer, delimiter);
            stream_to_sink(driver, params, query, &mut sink, &mut progress).await?;
            sink.finish()?;
        }
        ExportFormat::Json => {
            let mut sink = JsonSink::new(writer);
            stream_to_sink(driver, params, query, &mut sink, &mut progress).await?;
            sink.finish()?;
        }
        ExportFormat::Markdown => {
            let mut sink = MarkdownSink::new(writer);
            stream_to_sink(driver, params, query, &mut sink, &mut progress).await?;
            sink.finish()?;
        }
        ExportFormat::Parquet => {
            let mut sink = ParquetSink::new(writer);
            stream_typed_to_sink(driver, params, query, &mut sink, &mut progress).await?;
            sink.finish()?;
        }
    }

    progress.finish();
    Ok(())
}

async fn stream_to_sink<S, F>(
    driver: &str,
    params: &ConnectionParams,
    query: &str,
    sink: &mut S,
    progress: &mut ProgressEmitter<F>,
) -> Result<(), String>
where
    S: RowSink + Send,
    F: FnMut(u64) + Send,
{
    let mut on_row = |headers: &[String], values: &[Value]| -> Result<(), String> {
        sink.write_row(headers, values)?;
        progress.tick();
        Ok(())
    };

    match driver {
        "mysql" => mysql::export::stream_query(params, query, &mut on_row).await,
        "postgres" => postgres::export::stream_query(params, query, &mut on_row).await,
        "sqlite" => sqlite::export::stream_query(params, query, &mut on_row).await,
        // External plugin drivers: page through the driver's own paginated
        // `execute_query` and forward every row to the sink.
        other => stream_query_via_plugin(other, params, query, &mut on_row).await,
    }
}

async fn stream_typed_to_sink<S, F>(
    driver: &str,
    params: &ConnectionParams,
    query: &str,
    sink: &mut S,
    progress: &mut ProgressEmitter<F>,
) -> Result<(), String>
where
    S: TypedRowSink + Send,
    F: FnMut(u64) + Send,
{
    let mut began = false;
    let mut on_row = |columns: &[ColMeta], values: &[TVal]| -> Result<(), String> {
        if !began {
            sink.begin(columns)?;
            began = true;
        }
        sink.write_row(values)?;
        progress.tick();
        Ok(())
    };

    let result = match driver {
        "mysql" => mysql::export::stream_typed_query(params, query, &mut on_row).await,
        "postgres" => postgres::export::stream_typed_query(params, query, &mut on_row).await,
        "sqlite" => sqlite::export::stream_typed_query(params, query, &mut on_row).await,
        other => stream_typed_via_plugin(other, params, query, &mut on_row).await,
    };

    // Empty result sets never call on_row — still emit a valid empty Parquet file
    // when the query returns zero rows. Callers that know column metadata could
    // begin earlier; without a row we cannot invent a schema, so finish alone
    // would fail. Surface a clear error instead of a corrupt file.
    if result.is_ok() && !began {
        return Err(
            "Query returned no rows; Parquet export needs at least one row to infer columns"
                .into(),
        );
    }
    result
}

/// Streams a query for an external plugin driver by repeatedly calling its
/// `execute_query` with the driver's pagination, forwarding each row to
/// `on_row`. Built-in drivers stream directly from the database; plugins only
/// expose paged query execution over JSON-RPC, so we drive that here.
async fn stream_query_via_plugin<F>(
    driver_id: &str,
    params: &ConnectionParams,
    query: &str,
    mut on_row: F,
) -> Result<(), String>
where
    F: FnMut(&[String], &[Value]) -> Result<(), String> + Send,
{
    const PAGE_SIZE: u32 = 1000;

    let driver = crate::drivers::registry::get_driver(driver_id)
        .await
        .ok_or_else(|| format!("Unsupported driver for export: {driver_id}"))?;

    let mut page: u32 = 1;
    loop {
        let result = driver
            .execute_query(params, query, Some(PAGE_SIZE), page, None)
            .await?;

        for row in &result.rows {
            on_row(&result.columns, row)?;
        }

        let fetched = result.rows.len() as u32;
        let has_more = result
            .pagination
            .as_ref()
            .map(|p| p.has_more)
            .unwrap_or(fetched >= PAGE_SIZE);

        if fetched == 0 || !has_more {
            break;
        }
        page += 1;
    }

    Ok(())
}

/// Plugin Parquet path: weak typing from JSON cells (no declared column types
/// on [`QueryResult`]). Logs a warning once when falling back.
async fn stream_typed_via_plugin<F>(
    driver_id: &str,
    params: &ConnectionParams,
    query: &str,
    mut on_row: F,
) -> Result<(), String>
where
    F: FnMut(&[ColMeta], &[TVal]) -> Result<(), String> + Send,
{
    const PAGE_SIZE: u32 = 1000;

    let driver = crate::drivers::registry::get_driver(driver_id)
        .await
        .ok_or_else(|| format!("Unsupported driver for export: {driver_id}"))?;

    let mut page: u32 = 1;
    let mut columns: Option<Vec<ColMeta>> = None;
    let mut warned = false;

    loop {
        let result = driver
            .execute_query(params, query, Some(PAGE_SIZE), page, None)
            .await?;

        if columns.is_none() && !result.columns.is_empty() {
            // Infer kinds from the first non-null cell per column across the page.
            let mut metas: Vec<ColMeta> = result
                .columns
                .iter()
                .map(|name| column_meta_from_type_name(name, "TEXT"))
                .collect();
            for (col_idx, meta) in metas.iter_mut().enumerate() {
                for row in &result.rows {
                    if let Some(cell) = row.get(col_idx) {
                        if !cell.is_null() {
                            meta.kind = infer_kind_from_json(cell);
                            break;
                        }
                    }
                }
            }
            if !warned {
                log::warn!(
                    "[export] plugin driver '{driver_id}' Parquet export uses weak JSON typing"
                );
                warned = true;
            }
            columns = Some(metas);
        }

        if let Some(cols) = columns.as_ref() {
            for row in &result.rows {
                let values: Result<Vec<TVal>, String> = cols
                    .iter()
                    .enumerate()
                    .map(|(i, meta)| {
                        let cell = row.get(i).unwrap_or(&Value::Null);
                        typed_value_from_json(cell, meta.kind).or_else(|err| {
                            log::debug!(
                                "[export] plugin coerce fallback for '{}': {err}",
                                meta.name
                            );
                            // Safe fallback: stringify rather than abort the whole export.
                            Ok(TVal::Utf8(match cell {
                                Value::String(s) => s.clone(),
                                Value::Null => String::new(),
                                other => other.to_string(),
                            }))
                        })
                    })
                    .collect();
                on_row(cols, &values?)?;
            }
        }

        let fetched = result.rows.len() as u32;
        let has_more = result
            .pagination
            .as_ref()
            .map(|p| p.has_more)
            .unwrap_or(fetched >= PAGE_SIZE);

        if fetched == 0 || !has_more {
            break;
        }
        page += 1;
    }

    Ok(())
}
