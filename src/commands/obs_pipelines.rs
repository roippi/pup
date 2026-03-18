use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use datadog_api_client::datadogV2::api_observability_pipelines::{
    ListPipelinesOptionalParams, ObservabilityPipelinesAPI,
};
#[cfg(not(target_arch = "wasm32"))]
use datadog_api_client::datadogV2::model::{ObservabilityPipeline, ObservabilityPipelineSpec};

#[cfg(not(target_arch = "wasm32"))]
use crate::client;
use crate::config::Config;
use crate::formatter;
#[cfg(not(target_arch = "wasm32"))]
use crate::util;

#[cfg(not(target_arch = "wasm32"))]
fn make_api(cfg: &Config) -> ObservabilityPipelinesAPI {
    // Observability Pipelines does not support OAuth — API key auth only.
    ObservabilityPipelinesAPI::with_config(client::make_dd_config(cfg))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list(cfg: &Config, limit: i64) -> Result<()> {
    let api = make_api(cfg);
    let params = ListPipelinesOptionalParams::default().page_size(limit);
    let resp = api
        .list_pipelines(params)
        .await
        .map_err(|e| anyhow::anyhow!("failed to list pipelines: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn list(cfg: &Config, _limit: i64) -> Result<()> {
    let data = crate::api::get(
        cfg,
        "/api/v2/remote_config/products/obs_pipelines/pipelines",
        &[],
    )
    .await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get(cfg: &Config, pipeline_id: &str) -> Result<()> {
    let api = make_api(cfg);
    let resp = api
        .get_pipeline(pipeline_id.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("failed to get pipeline: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn get(cfg: &Config, pipeline_id: &str) -> Result<()> {
    let data = crate::api::get(
        cfg,
        &format!("/api/v2/remote_config/products/obs_pipelines/pipelines/{pipeline_id}"),
        &[],
    )
    .await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn create(cfg: &Config, file: &str) -> Result<()> {
    let body: ObservabilityPipelineSpec = util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .create_pipeline(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create pipeline: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn create(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = crate::util::read_json_file(file)?;
    let data = crate::api::post(
        cfg,
        "/api/v2/remote_config/products/obs_pipelines/pipelines",
        &body,
    )
    .await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn update(cfg: &Config, pipeline_id: &str, file: &str) -> Result<()> {
    let body: ObservabilityPipeline = util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .update_pipeline(pipeline_id.to_string(), body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to update pipeline: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn update(cfg: &Config, pipeline_id: &str, file: &str) -> Result<()> {
    let body: serde_json::Value = crate::util::read_json_file(file)?;
    let data = crate::api::put(
        cfg,
        &format!("/api/v2/remote_config/products/obs_pipelines/pipelines/{pipeline_id}"),
        &body,
    )
    .await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn delete(cfg: &Config, pipeline_id: &str) -> Result<()> {
    let api = make_api(cfg);
    api.delete_pipeline(pipeline_id.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("failed to delete pipeline: {e:?}"))?;
    eprintln!("Pipeline {pipeline_id} deleted.");
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn delete(cfg: &Config, pipeline_id: &str) -> Result<()> {
    crate::api::delete(
        cfg,
        &format!("/api/v2/remote_config/products/obs_pipelines/pipelines/{pipeline_id}"),
    )
    .await?;
    eprintln!("Pipeline {pipeline_id} deleted.");
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn validate(cfg: &Config, file: &str) -> Result<()> {
    let body: ObservabilityPipelineSpec = util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .validate_pipeline(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to validate pipeline: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn validate(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = crate::util::read_json_file(file)?;
    let data = crate::api::post(
        cfg,
        "/api/v2/remote_config/products/obs_pipelines/pipelines/validate",
        &body,
    )
    .await?;
    crate::formatter::output(cfg, &data)
}
