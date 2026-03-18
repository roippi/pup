use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use datadog_api_client::datadogV2::api_cloud_cost_management::CloudCostManagementAPI;
#[cfg(not(target_arch = "wasm32"))]
use datadog_api_client::datadogV2::api_usage_metering::{
    GetCostByOrgOptionalParams, GetMonthlyCostAttributionOptionalParams,
    GetProjectedCostOptionalParams, UsageMeteringAPI as UsageMeteringV2API,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::client;
use crate::config::Config;
use crate::formatter;
use crate::util;

#[cfg(not(target_arch = "wasm32"))]
pub async fn projected(cfg: &Config) -> Result<()> {
    let dd_cfg = client::make_dd_config(cfg);
    let api = match client::make_bearer_client(cfg) {
        Some(c) => UsageMeteringV2API::with_client_and_config(dd_cfg, c),
        None => UsageMeteringV2API::with_config(dd_cfg),
    };
    let resp = api
        .get_projected_cost(GetProjectedCostOptionalParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("failed to get projected cost: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn projected(cfg: &Config) -> Result<()> {
    let data = crate::api::get(cfg, "/api/v2/usage/projected_cost", &[]).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn by_org(cfg: &Config, start_month: String, end_month: Option<String>) -> Result<()> {
    let dd_cfg = client::make_dd_config(cfg);
    let api = match client::make_bearer_client(cfg) {
        Some(c) => UsageMeteringV2API::with_client_and_config(dd_cfg, c),
        None => UsageMeteringV2API::with_config(dd_cfg),
    };

    let start_dt =
        chrono::DateTime::from_timestamp_millis(util::parse_time_to_unix_millis(&start_month)?)
            .unwrap();

    let mut params = GetCostByOrgOptionalParams::default();
    if let Some(e) = end_month {
        let end_dt =
            chrono::DateTime::from_timestamp_millis(util::parse_time_to_unix_millis(&e)?).unwrap();
        params = params.end_month(end_dt);
    }

    let resp = api
        .get_cost_by_org(start_dt, params)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get cost by org: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn by_org(cfg: &Config, start_month: String, end_month: Option<String>) -> Result<()> {
    let start_dt =
        chrono::DateTime::from_timestamp_millis(util::parse_time_to_unix_millis(&start_month)?)
            .unwrap();
    let mut query = vec![("start_month", start_dt.to_rfc3339())];
    if let Some(e) = end_month {
        let end_dt =
            chrono::DateTime::from_timestamp_millis(util::parse_time_to_unix_millis(&e)?).unwrap();
        query.push(("end_month", end_dt.to_rfc3339()));
    }
    let data = crate::api::get(cfg, "/api/v2/usage/cost_by_org", &query).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn attribution(cfg: &Config, start: String, fields: Option<String>) -> Result<()> {
    let dd_cfg = client::make_dd_config(cfg);
    let api = match client::make_bearer_client(cfg) {
        Some(c) => UsageMeteringV2API::with_client_and_config(dd_cfg, c),
        None => UsageMeteringV2API::with_config(dd_cfg),
    };

    let start_dt =
        chrono::DateTime::from_timestamp_millis(util::parse_time_to_unix_millis(&start)?).unwrap();

    let fields_str = fields.unwrap_or_else(|| "*".to_string());
    let params = GetMonthlyCostAttributionOptionalParams::default();

    let resp = api
        .get_monthly_cost_attribution(start_dt, fields_str, params)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get cost attribution: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn attribution(cfg: &Config, start: String, fields: Option<String>) -> Result<()> {
    let start_dt =
        chrono::DateTime::from_timestamp_millis(util::parse_time_to_unix_millis(&start)?).unwrap();
    let fields_str = fields.unwrap_or_else(|| "*".to_string());
    let query = vec![
        ("start_month", start_dt.to_rfc3339()),
        ("fields", fields_str),
    ];
    let data = crate::api::get(cfg, "/api/v2/cost_by_tag/monthly_cost_attribution", &query).await?;
    crate::formatter::output(cfg, &data)
}

// ---- Cloud Cost Management — AWS CUR Config ----

#[cfg(not(target_arch = "wasm32"))]
fn make_ccm_api(cfg: &Config) -> CloudCostManagementAPI {
    let dd_cfg = client::make_dd_config(cfg);
    match client::make_bearer_client(cfg) {
        Some(c) => CloudCostManagementAPI::with_client_and_config(dd_cfg, c),
        None => CloudCostManagementAPI::with_config(dd_cfg),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn aws_config_list(cfg: &Config) -> Result<()> {
    let api = make_ccm_api(cfg);
    let resp = api
        .list_cost_awscur_configs()
        .await
        .map_err(|e| anyhow::anyhow!("failed to list AWS CUR configs: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn aws_config_list(cfg: &Config) -> Result<()> {
    let data = crate::api::get(cfg, "/api/v2/cost/aws_cur_config", &[]).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn aws_config_get(cfg: &Config, id: i64) -> Result<()> {
    let api = make_ccm_api(cfg);
    let resp = api
        .get_cost_awscur_config(id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get AWS CUR config: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn aws_config_get(cfg: &Config, id: i64) -> Result<()> {
    let data = crate::api::get(cfg, &format!("/api/v2/cost/aws_cur_config/{id}"), &[]).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn aws_config_create(cfg: &Config, file: &str) -> Result<()> {
    let body: datadog_api_client::datadogV2::model::AwsCURConfigPostRequest =
        util::read_json_file(file)?;
    let api = make_ccm_api(cfg);
    let resp = api
        .create_cost_awscur_config(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create AWS CUR config: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn aws_config_create(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = util::read_json_file(file)?;
    let data = crate::api::post(cfg, "/api/v2/cost/aws_cur_config", &body).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn aws_config_delete(cfg: &Config, id: i64) -> Result<()> {
    let api = make_ccm_api(cfg);
    api.delete_cost_awscur_config(id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to delete AWS CUR config: {e:?}"))?;
    eprintln!("AWS CUR config {id} deleted.");
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn aws_config_delete(cfg: &Config, id: i64) -> Result<()> {
    crate::api::delete(cfg, &format!("/api/v2/cost/aws_cur_config/{id}")).await?;
    eprintln!("AWS CUR config {id} deleted.");
    Ok(())
}

// ---- Cloud Cost Management — Azure UC Config ----

#[cfg(not(target_arch = "wasm32"))]
pub async fn azure_config_list(cfg: &Config) -> Result<()> {
    let api = make_ccm_api(cfg);
    let resp = api
        .list_cost_azure_uc_configs()
        .await
        .map_err(|e| anyhow::anyhow!("failed to list Azure UC configs: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn azure_config_list(cfg: &Config) -> Result<()> {
    let data = crate::api::get(cfg, "/api/v2/cost/azure_uc_config", &[]).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn azure_config_get(cfg: &Config, id: i64) -> Result<()> {
    let api = make_ccm_api(cfg);
    let resp = api
        .get_cost_azure_uc_config(id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get Azure UC config: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn azure_config_get(cfg: &Config, id: i64) -> Result<()> {
    let data = crate::api::get(cfg, &format!("/api/v2/cost/azure_uc_config/{id}"), &[]).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn azure_config_create(cfg: &Config, file: &str) -> Result<()> {
    let body: datadog_api_client::datadogV2::model::AzureUCConfigPostRequest =
        util::read_json_file(file)?;
    let api = make_ccm_api(cfg);
    let resp = api
        .create_cost_azure_uc_configs(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create Azure UC config: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn azure_config_create(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = util::read_json_file(file)?;
    let data = crate::api::post(cfg, "/api/v2/cost/azure_uc_config", &body).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn azure_config_delete(cfg: &Config, id: i64) -> Result<()> {
    let api = make_ccm_api(cfg);
    api.delete_cost_azure_uc_config(id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to delete Azure UC config: {e:?}"))?;
    eprintln!("Azure UC config {id} deleted.");
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn azure_config_delete(cfg: &Config, id: i64) -> Result<()> {
    crate::api::delete(cfg, &format!("/api/v2/cost/azure_uc_config/{id}")).await?;
    eprintln!("Azure UC config {id} deleted.");
    Ok(())
}

// ---- Cloud Cost Management — GCP Usage Cost Config ----

#[cfg(not(target_arch = "wasm32"))]
pub async fn gcp_config_list(cfg: &Config) -> Result<()> {
    let api = make_ccm_api(cfg);
    let resp = api
        .list_cost_gcp_usage_cost_configs()
        .await
        .map_err(|e| anyhow::anyhow!("failed to list GCP usage cost configs: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn gcp_config_list(cfg: &Config) -> Result<()> {
    let data = crate::api::get(cfg, "/api/v2/cost/gcp_uc_config", &[]).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn gcp_config_get(cfg: &Config, id: i64) -> Result<()> {
    let api = make_ccm_api(cfg);
    let resp = api
        .get_cost_gcp_usage_cost_config(id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get GCP usage cost config: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn gcp_config_get(cfg: &Config, id: i64) -> Result<()> {
    let data = crate::api::get(cfg, &format!("/api/v2/cost/gcp_uc_config/{id}"), &[]).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn gcp_config_create(cfg: &Config, file: &str) -> Result<()> {
    let body: datadog_api_client::datadogV2::model::GCPUsageCostConfigPostRequest =
        util::read_json_file(file)?;
    let api = make_ccm_api(cfg);
    let resp = api
        .create_cost_gcp_usage_cost_config(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create GCP usage cost config: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn gcp_config_create(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = util::read_json_file(file)?;
    let data = crate::api::post(cfg, "/api/v2/cost/gcp_uc_config", &body).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn gcp_config_delete(cfg: &Config, id: i64) -> Result<()> {
    let api = make_ccm_api(cfg);
    api.delete_cost_gcp_usage_cost_config(id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to delete GCP usage cost config: {e:?}"))?;
    eprintln!("GCP usage cost config {id} deleted.");
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn gcp_config_delete(cfg: &Config, id: i64) -> Result<()> {
    crate::api::delete(cfg, &format!("/api/v2/cost/gcp_uc_config/{id}")).await?;
    eprintln!("GCP usage cost config {id} deleted.");
    Ok(())
}
