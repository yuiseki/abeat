use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_repo_agents_rules: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_skills_summary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_recent_runs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amem_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amem_today: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amem_owner_profile: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amem_open_tasks: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amem_recent_activity_period: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConfig {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_inline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDefinitionFile {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub kind: String,
    pub enabled: bool,
    pub schedule_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub every: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown: Option<String>,
    pub agent: String,
    pub workspace: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    pub no_op_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
    pub context: ContextConfig,
    pub action: ActionConfig,
}

#[derive(Debug, Clone)]
pub struct LoadedJob {
    pub source_name: String,
    #[allow(dead_code)]
    pub source_path: PathBuf,
    pub source_modified_at: Option<SystemTime>,
    pub def: JobDefinitionFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDefinition {
    pub id: Option<String>,
    pub description: Option<String>,
    pub kind: Option<String>,
    pub enabled: Option<bool>,
    pub schedule_kind: Option<String>,
    pub every: Option<String>,
    pub cron: Option<String>,
    pub agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub source: String,
    pub id: String,
    pub description: Option<String>,
    pub kind: Option<String>,
    pub enabled: Option<bool>,
    pub schedule_kind: Option<String>,
    pub every: Option<String>,
    pub cron: Option<String>,
    pub agent: Option<String>,
}

impl JobSummary {
    pub fn schedule_display(&self) -> String {
        match self.schedule_kind.as_deref() {
            Some("every") => self
                .every
                .as_ref()
                .map(|v| format!("every {v}"))
                .unwrap_or_else(|| "every ?".to_string()),
            Some("cron") => self
                .cron
                .as_ref()
                .map(|v| format!("cron {v}"))
                .unwrap_or_else(|| "cron ?".to_string()),
            Some(other) => other.to_string(),
            None => "-".to_string(),
        }
    }
}

pub fn load_job_summaries(jobs_dir: &Path) -> Result<Vec<JobSummary>> {
    if !jobs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in
        fs::read_dir(jobs_dir).with_context(|| format!("failed to read {}", jobs_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let parsed: JobDefinition =
            toml::from_str(&raw).with_context(|| format!("invalid TOML in {}", path.display()))?;

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let id = parsed.id.clone().unwrap_or_else(|| stem.clone());

        out.push(JobSummary {
            source: stem,
            id,
            description: parsed.description,
            kind: parsed.kind,
            enabled: parsed.enabled,
            schedule_kind: parsed.schedule_kind,
            every: parsed.every,
            cron: parsed.cron,
            agent: parsed.agent,
        });
    }

    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub fn load_jobs(jobs_dir: &Path) -> Result<Vec<LoadedJob>> {
    if !jobs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in
        fs::read_dir(jobs_dir).with_context(|| format!("failed to read {}", jobs_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let source_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let def: JobDefinitionFile =
            toml::from_str(&raw).with_context(|| format!("invalid TOML in {}", path.display()))?;
        let source_modified_at = fs::metadata(&path).and_then(|m| m.modified()).ok();
        out.push(LoadedJob {
            source_name,
            source_path: path,
            source_modified_at,
            def,
        });
    }

    out.sort_by(|a, b| a.def.id.cmp(&b.def.id));
    Ok(out)
}

pub fn load_job_by_id(jobs_dir: &Path, id: &str) -> Result<LoadedJob> {
    let path = jobs_dir.join(format!("{id}.toml"));
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let def: JobDefinitionFile =
        toml::from_str(&raw).with_context(|| format!("invalid TOML in {}", path.display()))?;
    let source_modified_at = fs::metadata(&path).and_then(|m| m.modified()).ok();
    Ok(LoadedJob {
        source_name: id.to_string(),
        source_path: path,
        source_modified_at,
        def,
    })
}
