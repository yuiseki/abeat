use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobsStateFile {
    #[serde(default)]
    pub jobs: BTreeMap<String, JobRuntimeState>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobRuntimeState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    #[serde(default)]
    pub fail_count: u32,
}

pub fn load_jobs_state(path: &Path) -> Result<JobsStateFile> {
    if !path.exists() {
        return Ok(JobsStateFile::default());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(JobsStateFile::default());
    }
    let parsed: JobsStateFile = serde_json::from_str(&raw)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;
    Ok(parsed)
}

pub fn save_jobs_state(path: &Path, state: &JobsStateFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(state)?;
    fs::write(path, format!("{body}\n"))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}
