use anyhow::Result;
use datadog_api_client::datadogV2::api_audit::{
    AuditAPI, ListAuditLogsOptionalParams, SearchAuditLogsOptionalParams,
};
use datadog_api_client::datadogV2::model::{
    AuditLogsQueryFilter, AuditLogsQueryPageOptions, AuditLogsSearchEventsRequest, AuditLogsSort,
};

use crate::client;
use crate::config::Config;
use crate::formatter;
use crate::util;

pub async fn list(
    cfg: &Config,
    from: String,
    to: String,
    limit: i32,
    cursor: Option<String>,
) -> Result<()> {
    let dd_cfg = client::make_dd_config(cfg);
    let api = match client::make_bearer_client(cfg) {
        Some(c) => AuditAPI::with_client_and_config(dd_cfg, c),
        None => AuditAPI::with_config(dd_cfg),
    };

    let from_dt =
        chrono::DateTime::from_timestamp_millis(util::parse_time_to_unix_millis(&from)?).unwrap();
    let to_dt =
        chrono::DateTime::from_timestamp_millis(util::parse_time_to_unix_millis(&to)?).unwrap();

    let mut params = ListAuditLogsOptionalParams::default()
        .filter_from(from_dt)
        .filter_to(to_dt)
        .page_limit(limit);
    if let Some(c) = cursor {
        params = params.page_cursor(c);
    }

    let resp = api
        .list_audit_logs(params)
        .await
        .map_err(|e| anyhow::anyhow!("failed to list audit logs: {e:?}"))?;
    let raw = serde_json::to_value(&resp)?;
    formatter::output_with_raw(cfg, &resp, &raw)
}

pub async fn search(
    cfg: &Config,
    query: String,
    from: String,
    to: String,
    limit: i32,
    cursor: Option<String>,
) -> Result<()> {
    let dd_cfg = client::make_dd_config(cfg);
    let api = match client::make_bearer_client(cfg) {
        Some(c) => AuditAPI::with_client_and_config(dd_cfg, c),
        None => AuditAPI::with_config(dd_cfg),
    };

    let from_ms = util::parse_time_to_unix_millis(&from)?;
    let to_ms = util::parse_time_to_unix_millis(&to)?;

    let from_str = chrono::DateTime::from_timestamp_millis(from_ms)
        .unwrap()
        .to_rfc3339();
    let to_str = chrono::DateTime::from_timestamp_millis(to_ms)
        .unwrap()
        .to_rfc3339();

    let mut page = AuditLogsQueryPageOptions::new().limit(limit);
    if let Some(c) = cursor {
        page = page.cursor(c);
    }

    let body = AuditLogsSearchEventsRequest::new()
        .filter(
            AuditLogsQueryFilter::new()
                .query(query)
                .from(from_str)
                .to(to_str),
        )
        .page(page)
        .sort(AuditLogsSort::TIMESTAMP_DESCENDING);

    let params = SearchAuditLogsOptionalParams::default().body(body);
    let resp = api
        .search_audit_logs(params)
        .await
        .map_err(|e| anyhow::anyhow!("failed to search audit logs: {e:?}"))?;
    let raw = serde_json::to_value(&resp)?;
    formatter::output_with_raw(cfg, &resp, &raw)
}
