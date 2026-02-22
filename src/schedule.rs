use std::str::FromStr;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local, Utc};
use cron::Schedule;

use crate::jobs::LoadedJob;
use crate::state::JobRuntimeState;

pub fn is_due(
    job: &LoadedJob,
    state: Option<&JobRuntimeState>,
    now: DateTime<Utc>,
) -> Result<bool> {
    if !job.def.enabled {
        return Ok(false);
    }

    match job.def.schedule_kind.as_str() {
        "every" => is_due_every(job, state, now),
        "cron" => is_due_cron(job, state, now),
        other => bail!(
            "unsupported schedule_kind `{other}` for job `{}`",
            job.def.id
        ),
    }
}

fn is_due_every(
    job: &LoadedJob,
    state: Option<&JobRuntimeState>,
    now: DateTime<Utc>,
) -> Result<bool> {
    let every = job
        .def
        .every
        .as_deref()
        .with_context(|| format!("job `{}` is missing `every`", job.def.id))?;
    let duration = humantime::parse_duration(every)
        .with_context(|| format!("job `{}` has invalid duration `{every}`", job.def.id))?;

    let Some(state) = state else {
        return Ok(true);
    };
    let Some(last_finished_at) = state.last_finished_at.as_deref() else {
        return Ok(true);
    };
    let last_finished = parse_rfc3339_utc(last_finished_at)
        .with_context(|| format!("job `{}` has invalid state timestamp", job.def.id))?;
    let chrono_duration = chrono::Duration::from_std(duration)
        .with_context(|| format!("job `{}` duration is out of range", job.def.id))?;
    Ok(now >= last_finished + chrono_duration)
}

fn is_due_cron(
    job: &LoadedJob,
    state: Option<&JobRuntimeState>,
    now: DateTime<Utc>,
) -> Result<bool> {
    let cron_expr = job
        .def
        .cron
        .as_deref()
        .with_context(|| format!("job `{}` is missing `cron`", job.def.id))?;
    let schedule = Schedule::from_str(&normalize_cron_expr(cron_expr)?)
        .with_context(|| format!("job `{}` has invalid cron `{cron_expr}`", job.def.id))?;

    let marker_utc = match state.and_then(|s| s.last_finished_at.as_deref()) {
        Some(ts) => parse_rfc3339_utc(ts)
            .with_context(|| format!("job `{}` has invalid state timestamp", job.def.id))?,
        None => job
            .source_modified_at
            .map(DateTime::<Utc>::from)
            .unwrap_or(now),
    };
    let marker = marker_utc.with_timezone(&Local);
    let now_local = now.with_timezone(&Local);

    let next = schedule.after(&marker).next();
    Ok(matches!(next, Some(next_at) if next_at <= now_local))
}

fn normalize_cron_expr(expr: &str) -> Result<String> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    match parts.len() {
        5 => Ok(format!("0 {}", parts.join(" "))), // prepend seconds
        6 | 7 => Ok(expr.to_string()),
        _ => bail!("cron expression must have 5, 6, or 7 fields"),
    }
}

pub fn parse_rfc3339_utc(s: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(s)?.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::{ActionConfig, ContextConfig, JobDefinitionFile, LoadedJob};
    use crate::state::JobRuntimeState;
    use std::path::PathBuf;

    fn sample_job_every() -> LoadedJob {
        LoadedJob {
            source_name: "sample".to_string(),
            source_path: PathBuf::from("/tmp/sample.toml"),
            source_modified_at: None,
            def: JobDefinitionFile {
                id: "sample".to_string(),
                kind: "heartbeat_check".to_string(),
                enabled: true,
                schedule_kind: "every".to_string(),
                every: Some("1h".to_string()),
                cron: None,
                cooldown: None,
                agent: "codex".to_string(),
                workspace: "/tmp".to_string(),
                skills: vec![],
                no_op_token: "HEARTBEAT_OK".to_string(),
                timeout: None,
                context: ContextConfig {
                    timezone: None,
                    include_repo_agents_rules: None,
                    include_skills_summary: None,
                    include_recent_runs: None,
                    amem_mode: None,
                    amem_today: None,
                    amem_owner_profile: None,
                    amem_open_tasks: None,
                    amem_recent_activity_period: None,
                    extra_files: vec![],
                },
                action: ActionConfig {
                    mode: "command".to_string(),
                    prompt_template: None,
                    prompt_inline: None,
                    command: Some("echo hi".to_string()),
                    shell: Some("bash".to_string()),
                },
            },
        }
    }

    #[test]
    fn every_without_state_is_due() {
        let now = Utc::now();
        assert!(is_due(&sample_job_every(), None, now).unwrap());
    }

    #[test]
    fn every_respects_last_finished_at() {
        let now = Utc::now();
        let state = JobRuntimeState {
            last_finished_at: Some((now - chrono::Duration::minutes(30)).to_rfc3339()),
            ..JobRuntimeState::default()
        };
        assert!(!is_due(&sample_job_every(), Some(&state), now).unwrap());
    }
}
