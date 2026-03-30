use anyhow::Result;
use datadog_api_client::datadogV2::api_reference_tables::{
    ListTablesOptionalParams, ReferenceTablesAPI,
};

use crate::client;
use crate::config::Config;
use crate::formatter;
use crate::util;

fn make_api(cfg: &Config) -> ReferenceTablesAPI {
    let dd_cfg = client::make_dd_config(cfg);
    match client::make_bearer_client(cfg) {
        Some(c) => ReferenceTablesAPI::with_client_and_config(dd_cfg, c),
        None => ReferenceTablesAPI::with_config(dd_cfg),
    }
}

pub async fn list(cfg: &Config, limit: i64, page_offset: Option<i64>) -> Result<()> {
    let api = make_api(cfg);
    let mut params = ListTablesOptionalParams::default().page_limit(limit);
    if let Some(o) = page_offset {
        params = params.page_offset(o);
    }
    let resp = api
        .list_tables(params)
        .await
        .map_err(|e| anyhow::anyhow!("failed to list reference tables: {e:?}"))?;
    let raw = serde_json::to_value(&resp)?;
    formatter::output_with_raw(cfg, &resp, &raw)
}

pub async fn get(cfg: &Config, table_id: &str) -> Result<()> {
    let api = make_api(cfg);
    let resp = api
        .get_table(table_id.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("failed to get reference table: {e:?}"))?;
    formatter::output(cfg, &resp)
}

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
