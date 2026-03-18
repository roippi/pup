use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use datadog_api_client::datadogV2::api_reference_tables::{
    ListTablesOptionalParams, ReferenceTablesAPI,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::client;
use crate::config::Config;
use crate::formatter;
#[cfg(not(target_arch = "wasm32"))]
use crate::util;

#[cfg(not(target_arch = "wasm32"))]
fn make_api(cfg: &Config) -> ReferenceTablesAPI {
    let dd_cfg = client::make_dd_config(cfg);
    match client::make_bearer_client(cfg) {
        Some(c) => ReferenceTablesAPI::with_client_and_config(dd_cfg, c),
        None => ReferenceTablesAPI::with_config(dd_cfg),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list(cfg: &Config, limit: i64) -> Result<()> {
    let api = make_api(cfg);
    let params = ListTablesOptionalParams::default().page_limit(limit);
    let resp = api
        .list_tables(params)
        .await
        .map_err(|e| anyhow::anyhow!("failed to list reference tables: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn list(cfg: &Config, limit: i64) -> Result<()> {
    let query = vec![("page[limit]", limit.to_string())];
    let data = crate::api::get(cfg, "/api/v2/reference-tables", &query).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get(cfg: &Config, table_id: &str) -> Result<()> {
    let api = make_api(cfg);
    let resp = api
        .get_table(table_id.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("failed to get reference table: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn get(cfg: &Config, table_id: &str) -> Result<()> {
    let data = crate::api::get(
        cfg,
        &format!("/api/v2/reference-tables/{table_id}"),
        &[],
    )
    .await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn create(cfg: &Config, file: &str) -> Result<()> {
    let body: datadog_api_client::datadogV2::model::CreateTableRequest =
        util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .create_reference_table(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create reference table: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn create(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = crate::util::read_json_file(file)?;
    let data = crate::api::post(cfg, "/api/v2/reference-tables", &body).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn batch_query(cfg: &Config, file: &str) -> Result<()> {
    let body: datadog_api_client::datadogV2::model::BatchRowsQueryRequest =
        util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .batch_rows_query(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to batch query reference table rows: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn batch_query(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = crate::util::read_json_file(file)?;
    let data = crate::api::post(cfg, "/api/v2/reference-tables/queries/batch-rows", &body).await?;
    crate::formatter::output(cfg, &data)
}
