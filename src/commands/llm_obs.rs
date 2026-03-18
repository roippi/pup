use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use datadog_api_client::datadogV2::api_llm_observability::{
    LLMObservabilityAPI, ListLLMObsDatasetsOptionalParams, ListLLMObsExperimentsOptionalParams,
    ListLLMObsProjectsOptionalParams,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::client;
use crate::config::Config;
use crate::formatter;
#[cfg(not(target_arch = "wasm32"))]
use crate::util;

#[cfg(not(target_arch = "wasm32"))]
fn make_api(cfg: &Config) -> LLMObservabilityAPI {
    let dd_cfg = client::make_dd_config(cfg);
    match client::make_bearer_client(cfg) {
        Some(c) => LLMObservabilityAPI::with_client_and_config(dd_cfg, c),
        None => LLMObservabilityAPI::with_config(dd_cfg),
    }
}

// ---- Projects ----

#[cfg(not(target_arch = "wasm32"))]
pub async fn projects_create(cfg: &Config, file: &str) -> Result<()> {
    let body: datadog_api_client::datadogV2::model::LLMObsProjectRequest =
        util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .create_llm_obs_project(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create LLM obs project: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn projects_create(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = crate::util::read_json_file(file)?;
    let data = crate::api::post(cfg, "/api/v2/llm-obs/v1/projects", &body).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn projects_list(cfg: &Config) -> Result<()> {
    let api = make_api(cfg);
    let resp = api
        .list_llm_obs_projects(ListLLMObsProjectsOptionalParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("failed to list LLM obs projects: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn projects_list(cfg: &Config) -> Result<()> {
    let data = crate::api::get(cfg, "/api/v2/llm-obs/v1/projects", &[]).await?;
    crate::formatter::output(cfg, &data)
}

// ---- Experiments ----

#[cfg(not(target_arch = "wasm32"))]
pub async fn experiments_create(cfg: &Config, file: &str) -> Result<()> {
    let body: datadog_api_client::datadogV2::model::LLMObsExperimentRequest =
        util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .create_llm_obs_experiment(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create LLM obs experiment: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn experiments_create(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = crate::util::read_json_file(file)?;
    let data = crate::api::post(cfg, "/api/v2/llm-obs/v1/experiments", &body).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn experiments_list(
    cfg: &Config,
    filter_project_id: Option<String>,
    filter_dataset_id: Option<String>,
) -> Result<()> {
    let api = make_api(cfg);
    let mut params = ListLLMObsExperimentsOptionalParams::default();
    if let Some(pid) = filter_project_id {
        params = params.filter_project_id(pid);
    }
    if let Some(did) = filter_dataset_id {
        params = params.filter_dataset_id(did);
    }
    let resp = api
        .list_llm_obs_experiments(params)
        .await
        .map_err(|e| anyhow::anyhow!("failed to list LLM obs experiments: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn experiments_list(
    cfg: &Config,
    filter_project_id: Option<String>,
    filter_dataset_id: Option<String>,
) -> Result<()> {
    let mut query = vec![];
    if let Some(pid) = &filter_project_id {
        query.push(("filter[project_id]", pid.clone()));
    }
    if let Some(did) = &filter_dataset_id {
        query.push(("filter[dataset_id]", did.clone()));
    }
    let data = crate::api::get(cfg, "/api/v2/llm-obs/v1/experiments", &query).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn experiments_update(cfg: &Config, experiment_id: &str, file: &str) -> Result<()> {
    let body: datadog_api_client::datadogV2::model::LLMObsExperimentUpdateRequest =
        util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .update_llm_obs_experiment(experiment_id.to_string(), body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to update LLM obs experiment: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn experiments_update(cfg: &Config, experiment_id: &str, file: &str) -> Result<()> {
    let body: serde_json::Value = crate::util::read_json_file(file)?;
    let data = crate::api::patch(
        cfg,
        &format!("/api/v2/llm-obs/v1/experiments/{experiment_id}"),
        &body,
    )
    .await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn experiments_delete(cfg: &Config, file: &str) -> Result<()> {
    let body: datadog_api_client::datadogV2::model::LLMObsDeleteExperimentsRequest =
        util::read_json_file(file)?;
    let api = make_api(cfg);
    api.delete_llm_obs_experiments(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to delete LLM obs experiments: {e:?}"))?;
    eprintln!("LLM obs experiments deleted.");
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn experiments_delete(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = crate::util::read_json_file(file)?;
    crate::api::post(cfg, "/api/v2/llm-obs/v1/experiments/delete", &body).await?;
    eprintln!("LLM obs experiments deleted.");
    Ok(())
}

// ---- Datasets ----

#[cfg(not(target_arch = "wasm32"))]
pub async fn datasets_create(cfg: &Config, project_id: &str, file: &str) -> Result<()> {
    let body: datadog_api_client::datadogV2::model::LLMObsDatasetRequest =
        util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .create_llm_obs_dataset(project_id.to_string(), body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create LLM obs dataset: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn datasets_create(cfg: &Config, project_id: &str, file: &str) -> Result<()> {
    let body: serde_json::Value = crate::util::read_json_file(file)?;
    let data = crate::api::post(
        cfg,
        &format!("/api/v2/llm-obs/v1/projects/{project_id}/datasets"),
        &body,
    )
    .await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn datasets_list(cfg: &Config, project_id: &str) -> Result<()> {
    let api = make_api(cfg);
    let resp = api
        .list_llm_obs_datasets(
            project_id.to_string(),
            ListLLMObsDatasetsOptionalParams::default(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to list LLM obs datasets: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn datasets_list(cfg: &Config, project_id: &str) -> Result<()> {
    let data = crate::api::get(
        cfg,
        &format!("/api/v2/llm-obs/v1/projects/{project_id}/datasets"),
        &[],
    )
    .await?;
    crate::formatter::output(cfg, &data)
}
