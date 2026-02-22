mod cli;
mod jobs;
mod paths;
mod schedule;
mod state;

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::Parser;
use serde::Serialize;

use cli::{
    AbeCommand, Cli, GetCommand, GetJobsArgs, GetSubcommand, JobKindArg, LogsArgs, SetCommand,
    SetJobsAddArgs, SetJobsCommand, SetSubcommand, WhichArgs,
};
use jobs::{
    JobDefinitionFile, JobSummary, LoadedJob, load_job_by_id, load_job_summaries, load_jobs,
};
use paths::{AbeatPaths, resolve_paths};
use state::{JobsStateFile, load_jobs_state, save_jobs_state};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let paths = resolve_paths()?;

    match cli.command {
        AbeCommand::Init => cmd_init(&paths),
        AbeCommand::Which(args) => cmd_which(&paths, args),
        AbeCommand::Tick { due } => cmd_tick(&paths, due),
        AbeCommand::Run { job_id } => cmd_run(&paths, &job_id),
        AbeCommand::Logs(args) => cmd_logs(&paths, args),
        AbeCommand::List(args) => cmd_list(&paths, args),
        AbeCommand::Get(cmd) => cmd_get(&paths, cmd),
        AbeCommand::Set(cmd) => cmd_set(&paths, cmd),
    }
}

fn cmd_init(paths: &AbeatPaths) -> Result<()> {
    ensure_layout(paths)?;
    println!("{}", paths.runtime_root.display());
    Ok(())
}

fn cmd_which(paths: &AbeatPaths, args: WhichArgs) -> Result<()> {
    if args.json {
        let payload = serde_json::json!({
            "config_root": paths.config_root,
            "runtime_root": paths.runtime_root,
            "jobs_dir": paths.jobs_dir(),
            "logs_dir": paths.logs_dir(),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    match args.target.as_deref() {
        Some("config") => println!("{}", paths.config_root.display()),
        Some("jobs") => println!("{}", paths.jobs_dir().display()),
        Some("runtime") | Some("state") => println!("{}", paths.runtime_root.display()),
        Some("logs") => println!("{}", paths.logs_dir().display()),
        Some(other) => bail!("unknown target for `abeat which`: {other}"),
        None => {
            println!("config: {}", paths.config_root.display());
            println!("runtime: {}", paths.runtime_root.display());
        }
    }

    Ok(())
}

fn cmd_tick(paths: &AbeatPaths, due: bool) -> Result<()> {
    if !due {
        bail!("`abeat tick` currently requires `--due`");
    }

    ensure_layout(paths)?;

    let mut state = load_jobs_state(&paths.state_file())?;
    let jobs = load_jobs(&paths.jobs_dir())?;
    let now = Utc::now();

    for job in jobs {
        if !job.def.enabled {
            continue;
        }

        match schedule::is_due(&job, state.jobs.get(&job.def.id), now) {
            Ok(true) => {
                if let Err(err) = run_loaded_job(paths, &mut state, &job, "tick") {
                    eprintln!("Job {} failed to run: {err:#}", job.def.id);
                }
            }
            Ok(false) => {}
            Err(err) => {
                eprintln!("Job {} due-check error: {err:#}", job.def.id);
            }
        }
    }

    save_jobs_state(&paths.state_file(), &state)?;
    Ok(())
}

fn cmd_run(paths: &AbeatPaths, job_id: &str) -> Result<()> {
    ensure_layout(paths)?;
    let mut state = load_jobs_state(&paths.state_file())?;
    let job = load_job_by_id(&paths.jobs_dir(), job_id)?;
    run_loaded_job(paths, &mut state, &job, "manual")?;
    save_jobs_state(&paths.state_file(), &state)?;
    Ok(())
}

fn cmd_logs(paths: &AbeatPaths, args: LogsArgs) -> Result<()> {
    let runs_file = paths.runs_file();
    if !runs_file.exists() {
        println!("No logs yet ({})", runs_file.display());
        return Ok(());
    }

    let content = fs::read_to_string(&runs_file)
        .with_context(|| format!("failed to read {}", runs_file.display()))?;

    let mut lines: Vec<&str> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if let Some(job_id) = args.job.as_deref() {
        lines.retain(|line| line.contains(&format!("\"job_id\":\"{job_id}\"")));
    }
    if let Some(status) = args.status.as_deref() {
        lines.retain(|line| line.contains(&format!("\"status\":\"{status}\"")));
    }
    if lines.is_empty() {
        println!("No logs yet ({})", runs_file.display());
        return Ok(());
    }
    let limit = args.limit.unwrap_or(20);
    let start = lines.len().saturating_sub(limit as usize);
    for line in &lines[start..] {
        println!("{line}");
    }
    Ok(())
}

fn cmd_list(paths: &AbeatPaths, args: GetJobsArgs) -> Result<()> {
    let jobs = load_job_summaries(&paths.jobs_dir())?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&jobs)?);
        return Ok(());
    }

    if jobs.is_empty() {
        println!("No jobs found in {}", paths.jobs_dir().display());
        return Ok(());
    }

    print_jobs_table(&jobs);
    Ok(())
}

fn cmd_get(paths: &AbeatPaths, cmd: GetCommand) -> Result<()> {
    match cmd.subcommand {
        GetSubcommand::Jobs(args) => cmd_list(paths, args),
        GetSubcommand::Job { id, json } => {
            let path = paths.jobs_dir().join(format!("{id}.toml"));
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            if json {
                let parsed: toml::Value = toml::from_str(&raw)
                    .with_context(|| format!("invalid TOML in {}", path.display()))?;
                println!("{}", serde_json::to_string_pretty(&parsed)?);
            } else {
                print!("{raw}");
                io::stdout().flush().ok();
            }
            Ok(())
        }
        GetSubcommand::Runs => {
            bail!("`abeat get runs` is not implemented yet; use `abeat logs` for now")
        }
    }
}

fn cmd_set(paths: &AbeatPaths, cmd: SetCommand) -> Result<()> {
    match cmd.subcommand {
        SetSubcommand::Jobs(job_args) => match job_args.subcommand {
            SetJobsCommand::Add(args) => cmd_set_jobs_add(paths, args),
            SetJobsCommand::Update { id } => {
                bail!(
                    "`abeat set jobs update {id}` is not implemented yet; edit the TOML file directly"
                )
            }
            SetJobsCommand::Enable { id } => {
                bail!(
                    "`abeat set jobs enable {id}` is not implemented yet; edit enabled=true in the TOML file"
                )
            }
            SetJobsCommand::Disable { id } => {
                bail!(
                    "`abeat set jobs disable {id}` is not implemented yet; edit enabled=false in the TOML file"
                )
            }
            SetJobsCommand::Rm { id } => bail!(
                "`abeat set jobs rm {id}` is not implemented yet; remove {} manually",
                paths.jobs_dir().join(format!("{id}.toml")).display()
            ),
        },
    }
}

fn cmd_set_jobs_add(paths: &AbeatPaths, args: SetJobsAddArgs) -> Result<()> {
    fs::create_dir_all(paths.jobs_dir())
        .with_context(|| format!("failed to create {}", paths.jobs_dir().display()))?;

    if !is_valid_job_id(&args.id) {
        bail!("invalid job id `{}`: use [a-zA-Z0-9_-]", args.id);
    }

    let (schedule_kind, every, cron) = match (args.every.as_deref(), args.cron.as_deref()) {
        (Some(every), None) => ("every".to_string(), Some(every.to_string()), None),
        (None, Some(cron)) => ("cron".to_string(), None, Some(cron.to_string())),
        (Some(_), Some(_)) => bail!("specify either --every or --cron, not both"),
        (None, None) => bail!("either --every or --cron is required"),
    };

    if args.exec_command.is_some()
        && (args.prompt_template.is_some() || args.prompt_inline.is_some())
    {
        bail!("--exec cannot be combined with --prompt-template/--prompt-inline");
    }

    let action = if let Some(command) = args.exec_command.as_deref() {
        jobs::ActionConfig {
            mode: "command".to_string(),
            prompt_template: None,
            prompt_inline: None,
            command: Some(command.to_string()),
            shell: Some(args.shell.clone().unwrap_or_else(|| "bash".to_string())),
        }
    } else if let Some(prompt_template) = args.prompt_template.as_deref() {
        jobs::ActionConfig {
            mode: "agent_cli".to_string(),
            prompt_template: Some(prompt_template.to_string()),
            prompt_inline: None,
            command: None,
            shell: None,
        }
    } else if let Some(prompt_inline) = args.prompt_inline.as_deref() {
        jobs::ActionConfig {
            mode: "agent_cli".to_string(),
            prompt_template: None,
            prompt_inline: Some(prompt_inline.to_string()),
            command: None,
            shell: None,
        }
    } else {
        jobs::ActionConfig {
            mode: "agent_cli".to_string(),
            prompt_template: Some("heartbeat-default".to_string()),
            prompt_inline: None,
            command: None,
            shell: None,
        }
    };

    let job = JobDefinitionFile {
        id: args.id.clone(),
        kind: match args.kind {
            JobKindArg::HeartbeatCheck => "heartbeat_check".to_string(),
            JobKindArg::ScheduledTask => "scheduled_task".to_string(),
        },
        enabled: true,
        schedule_kind,
        every,
        cron,
        cooldown: args.cooldown.clone(),
        agent: args.agent.clone(),
        workspace: args.workspace.display().to_string(),
        skills: args.skill.clone(),
        no_op_token: args
            .no_op_token
            .clone()
            .unwrap_or_else(|| "HEARTBEAT_OK".to_string()),
        timeout: args.timeout.clone(),
        context: jobs::ContextConfig {
            timezone: args.timezone.clone(),
            include_repo_agents_rules: Some(true),
            include_skills_summary: Some(true),
            include_recent_runs: Some(3),
            amem_mode: Some(args.amem_mode.clone()),
            amem_today: Some(args.amem_mode != "off"),
            amem_owner_profile: Some(args.amem_mode != "off"),
            amem_open_tasks: Some(args.amem_mode != "off"),
            amem_recent_activity_period: if args.amem_mode == "off" {
                None
            } else {
                Some("day".to_string())
            },
            extra_files: Vec::new(),
        },
        action,
    };

    let path = paths.jobs_dir().join(format!("{}.toml", job.id));
    if path.exists() {
        bail!(
            "job already exists: {} (use manual edit or future `abeat set jobs update`)",
            path.display()
        );
    }

    let toml = toml::to_string_pretty(&job).context("failed to serialize job definition")?;
    fs::write(&path, toml).with_context(|| format!("failed to write {}", path.display()))?;

    println!("{}", path.display());
    Ok(())
}

fn ensure_layout(paths: &AbeatPaths) -> Result<()> {
    for dir in paths.config_dirs() {
        fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    }
    for dir in paths.runtime_dirs() {
        fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    }

    ensure_file_if_missing(
        &paths.config_root.join("config.toml"),
        b"# abeat configuration\n# Edit as needed.\n",
    )?;
    ensure_file_if_missing(&paths.runs_file(), b"")?;
    ensure_file_if_missing(&paths.state_file(), b"{\n  \"jobs\": {}\n}\n")?;
    ensure_file_if_missing(&paths.runtime_root.join("state/runner.log"), b"")?;
    Ok(())
}

fn ensure_file_if_missing(path: &Path, content: &[u8]) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn run_loaded_job(
    paths: &AbeatPaths,
    state: &mut JobsStateFile,
    job: &LoadedJob,
    trigger: &str,
) -> Result<()> {
    let lock = match try_acquire_job_lock(paths, &job.def.id)? {
        Some(lock) => lock,
        None => {
            let now = Utc::now();
            let record = RunRecord {
                run_id: make_run_id(&job.def.id, now),
                job_id: job.def.id.clone(),
                source: job.source_name.clone(),
                status: "skipped_locked".to_string(),
                started_at: now.to_rfc3339(),
                ended_at: now.to_rfc3339(),
                agent: job.def.agent.clone(),
                trigger: trigger.to_string(),
                action_mode: job.def.action.mode.clone(),
                exit_code: None,
                no_op: false,
                stdout_log: None,
                stderr_log: None,
                message: Some("job lock already exists".to_string()),
            };
            append_run_record(&paths.runs_file(), &record)?;
            println!("Skipped (locked): {}", job.def.id);
            return Ok(());
        }
    };

    let started_at = Utc::now();
    let run_id = make_run_id(&job.def.id, started_at);
    let state_entry = state.jobs.entry(job.def.id.clone()).or_default();
    state_entry.last_run_id = Some(run_id.clone());
    state_entry.last_started_at = Some(started_at.to_rfc3339());

    let execution = execute_job(job);
    let ended_at = Utc::now();

    let (stdout_bytes, stderr_bytes, exit_code, status, no_op, message) = match execution {
        Ok(outcome) => {
            let status = if outcome.success {
                if outcome.no_op {
                    "no-op".to_string()
                } else {
                    "ok".to_string()
                }
            } else {
                "error".to_string()
            };
            (
                outcome.stdout,
                outcome.stderr,
                outcome.exit_code,
                status,
                outcome.no_op,
                outcome.message,
            )
        }
        Err(err) => (
            Vec::new(),
            format!("{}\n", err).into_bytes(),
            None,
            "error".to_string(),
            false,
            Some(err.to_string()),
        ),
    };

    let stdout_log = write_log_file(paths.stdout_logs_dir(), &run_id, &stdout_bytes)?;
    let stderr_log = write_log_file(paths.stderr_logs_dir(), &run_id, &stderr_bytes)?;

    let record = RunRecord {
        run_id: run_id.clone(),
        job_id: job.def.id.clone(),
        source: job.source_name.clone(),
        status: status.clone(),
        started_at: started_at.to_rfc3339(),
        ended_at: ended_at.to_rfc3339(),
        agent: job.def.agent.clone(),
        trigger: trigger.to_string(),
        action_mode: job.def.action.mode.clone(),
        exit_code,
        no_op,
        stdout_log: Some(stdout_log),
        stderr_log: Some(stderr_log),
        message,
    };
    append_run_record(&paths.runs_file(), &record)?;

    state_entry.last_finished_at = Some(ended_at.to_rfc3339());
    state_entry.last_status = Some(status.clone());
    if status == "error" {
        state_entry.fail_count = state_entry.fail_count.saturating_add(1);
    } else if status != "skipped_locked" {
        state_entry.fail_count = 0;
    }

    drop(lock);

    println!(
        "{}: {} ({})",
        job.def.id,
        status,
        record
            .message
            .as_deref()
            .unwrap_or_else(|| record.action_mode.as_str())
    );
    Ok(())
}

fn try_acquire_job_lock(paths: &AbeatPaths, job_id: &str) -> Result<Option<JobLock>> {
    fs::create_dir_all(paths.locks_dir())
        .with_context(|| format!("failed to create {}", paths.locks_dir().display()))?;
    let path = paths.locks_dir().join(format!("{job_id}.lock"));
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => {
            let _ = writeln!(file, "pid={}", std::process::id());
            let _ = writeln!(file, "started_at={}", Utc::now().to_rfc3339());
            Ok(Some(JobLock { path }))
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(None),
        Err(err) => Err(err).with_context(|| format!("failed to create lock {}", path.display())),
    }
}

struct JobLock {
    path: PathBuf,
}

impl Drop for JobLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

struct ExecutionOutcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: Option<i32>,
    success: bool,
    no_op: bool,
    message: Option<String>,
}

fn execute_job(job: &LoadedJob) -> Result<ExecutionOutcome> {
    match job.def.action.mode.as_str() {
        "command" => execute_command_job(job),
        "agent_cli" => bail!("agent_cli mode is not implemented yet for runtime execution"),
        other => bail!("unsupported action.mode `{other}`"),
    }
}

fn execute_command_job(job: &LoadedJob) -> Result<ExecutionOutcome> {
    let command = job
        .def
        .action
        .command
        .as_deref()
        .with_context(|| format!("job `{}` is missing action.command", job.def.id))?;
    let shell = job.def.action.shell.as_deref().unwrap_or("bash");

    let output = Command::new(shell)
        .arg("-lc")
        .arg(command)
        .current_dir(&job.def.workspace)
        .output()
        .with_context(|| format!("failed to execute shell `{shell}` for job `{}`", job.def.id))?;

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    let no_op = detect_no_op(&stdout_text, &job.def.no_op_token);
    let code = output.status.code();
    let message = if output.status.success() {
        None
    } else {
        Some(format!("command exited with status {}", output.status))
    };

    Ok(ExecutionOutcome {
        stdout: output.stdout,
        stderr: output.stderr,
        exit_code: code,
        success: output.status.success(),
        no_op,
        message,
    })
}

fn detect_no_op(stdout: &str, token: &str) -> bool {
    let lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    match (lines.first(), lines.last()) {
        (Some(first), Some(last)) => *first == token || *last == token,
        _ => false,
    }
}

fn write_log_file(dir: PathBuf, run_id: &str, bytes: &[u8]) -> Result<String> {
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let path = dir.join(format!("{run_id}.log"));
    fs::write(&path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path.display().to_string())
}

fn append_run_record(path: &Path, record: &RunRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let line = serde_json::to_string(record)?;
    writeln!(file, "{line}").with_context(|| format!("failed to append {}", path.display()))?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct RunRecord {
    run_id: String,
    job_id: String,
    source: String,
    status: String,
    started_at: String,
    ended_at: String,
    agent: String,
    trigger: String,
    action_mode: String,
    exit_code: Option<i32>,
    no_op: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout_log: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr_log: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

fn make_run_id(job_id: &str, now: chrono::DateTime<Utc>) -> String {
    format!("{}_{}", now.format("%Y%m%dT%H%M%SZ"), job_id)
}

fn print_jobs_table(jobs: &[JobSummary]) {
    println!(
        "{:<20} {:<16} {:<16} {:<8} {:<10} {:<12}",
        "ID", "KIND", "SCHEDULE", "ENABLED", "AGENT", "SOURCE"
    );
    for job in jobs {
        println!(
            "{:<20} {:<16} {:<16} {:<8} {:<10} {:<12}",
            truncate(&job.id, 20),
            truncate(job.kind.as_deref().unwrap_or("-"), 16),
            truncate(&job.schedule_display(), 16),
            if job.enabled.unwrap_or(true) {
                "yes"
            } else {
                "no"
            },
            truncate(job.agent.as_deref().unwrap_or("-"), 10),
            truncate(&job.source, 12),
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i + 1 >= max {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

fn is_valid_job_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_job_ids() {
        assert!(is_valid_job_id("inbox-check"));
        assert!(is_valid_job_id("weekly_review_1"));
        assert!(!is_valid_job_id("bad id"));
        assert!(!is_valid_job_id("bad/slash"));
        assert!(!is_valid_job_id(""));
    }

    #[test]
    fn truncates_long_text() {
        assert_eq!(truncate("abc", 5), "abc");
        assert_eq!(truncate("abcdef", 4), "abc…");
    }

    #[test]
    fn detects_noop_from_first_or_last_line() {
        assert!(detect_no_op("HEARTBEAT_OK\n", "HEARTBEAT_OK"));
        assert!(detect_no_op("work\nHEARTBEAT_OK\n", "HEARTBEAT_OK"));
        assert!(!detect_no_op("work\nok\n", "HEARTBEAT_OK"));
    }
}
