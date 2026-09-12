use futures::StreamExt;
use serde_json::Value;

use crate::export::{ColumnExportMeta, TypedValue};
use crate::models::ConnectionParams;
use crate::pool_manager::get_postgres_pool;

use super::export_typed::{column_meta_from_row, extract_typed};
use super::extract::extract_value;

/// Streams the rows produced by `query` against a PostgreSQL connection. See
/// the MySQL counterpart for the contract of `on_row`.
pub async fn stream_query<F>(
    params: &ConnectionParams,
    query: &str,
    mut on_row: F,
) -> Result<(), String>
where
    F: FnMut(&[String], &[Value]) -> Result<(), String> + Send,
{
    let pool = get_postgres_pool(params).await?;
    let client = pool
        .get()
        .await
        .map_err(|e| format!("failed to get postgres client: {:?}", e))?;

    let bind_params: Vec<i32> = vec![];
    let mut rows = std::pin::pin!(client
        .query_raw(query, &bind_params)
        .await
        .map_err(|e| format!("failed to execute postgres query: {:?}", e))?);

    let mut headers: Option<Vec<String>> = None;

    while let Some(row_res) = rows.next().await {
        let row = row_res.map_err(|e| e.to_string())?;

        if headers.is_none() {
            headers = Some(row.columns().iter().map(|c| c.name().to_string()).collect());
        }
        let h = headers.as_ref().expect("headers initialized");

        let values: Vec<Value> = (0..row.columns().len())
            .map(|i| extract_value(&row, i, None))
            .collect();

        on_row(h, &values)?;
    }

    Ok(())
}

/// Typed stream for Parquet export.
pub async fn stream_typed_query<F>(
    params: &ConnectionParams,
    query: &str,
    mut on_row: F,
) -> Result<(), String>
where
    F: FnMut(&[ColumnExportMeta], &[TypedValue]) -> Result<(), String> + Send,
{
    let pool = get_postgres_pool(params).await?;
    let client = pool
        .get()
        .await
        .map_err(|e| format!("failed to get postgres client: {:?}", e))?;

    let bind_params: Vec<i32> = vec![];
    let mut rows = std::pin::pin!(client
        .query_raw(query, &bind_params)
        .await
        .map_err(|e| format!("failed to execute postgres query: {:?}", e))?);

    let mut columns: Option<Vec<ColumnExportMeta>> = None;

    while let Some(row_res) = rows.next().await {
        let row = row_res.map_err(|e| e.to_string())?;

        if columns.is_none() {
            columns = Some(
                (0..row.columns().len())
                    .map(|i| column_meta_from_row(&row, i))
                    .collect(),
            );
        }
        let cols = columns.as_ref().expect("columns initialized");
        let values: Vec<TypedValue> = cols
            .iter()
            .enumerate()
            .map(|(i, meta)| extract_typed(&row, i, meta))
            .collect();
        on_row(cols, &values)?;
    }

    Ok(())
}
