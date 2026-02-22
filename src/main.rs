mod cli;
mod jobs;
mod paths;

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::Parser;
use cli::{
    AbeCommand, Cli, GetCommand, GetJobsArgs, GetSubcommand, JobKindArg, LogsArgs, SetCommand,
    SetJobsAddArgs, SetJobsCommand, SetSubcommand, WhichArgs,
};
use jobs::{JobDefinitionFile, JobSummary, load_job_summaries};
use paths::{AbeatPaths, resolve_paths};

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
        AbeCommand::Tick { due } => cmd_tick(due),
        AbeCommand::Run { job_id } => cmd_run(&job_id),
        AbeCommand::Logs(args) => cmd_logs(&paths, args),
        AbeCommand::List(args) => cmd_list(&paths, args),
        AbeCommand::Get(cmd) => cmd_get(&paths, cmd),
        AbeCommand::Set(cmd) => cmd_set(&paths, cmd),
    }
}

fn cmd_init(paths: &AbeatPaths) -> Result<()> {
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
    ensure_file_if_missing(&paths.runtime_root.join("logs/runs.jsonl"), b"")?;
    ensure_file_if_missing(
        &paths.runtime_root.join("state/jobs-state.json"),
        b"{\n  \"jobs\": {}\n}\n",
    )?;
    ensure_file_if_missing(&paths.runtime_root.join("state/runner.log"), b"")?;

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

fn cmd_tick(due: bool) -> Result<()> {
    if !due {
        bail!("`abeat tick` currently requires `--due`");
    }
    bail!("`abeat tick --due` is not implemented yet (CLI scaffold is ready)")
}

fn cmd_run(job_id: &str) -> Result<()> {
    bail!("`abeat run {job_id}` is not implemented yet (CLI scaffold is ready)")
}

fn cmd_logs(paths: &AbeatPaths, args: LogsArgs) -> Result<()> {
    let runs_file = paths.runtime_root.join("logs/runs.jsonl");
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
            SetJobsCommand::Update { id } => bail!(
                "`abeat set jobs update {id}` is not implemented yet; edit the TOML file directly"
            ),
            SetJobsCommand::Enable { id } => bail!(
                "`abeat set jobs enable {id}` is not implemented yet; edit enabled=true in the TOML file"
            ),
            SetJobsCommand::Disable { id } => bail!(
                "`abeat set jobs disable {id}` is not implemented yet; edit enabled=false in the TOML file"
            ),
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

    let action = if let Some(prompt_template) = args.prompt_template.as_deref() {
        jobs::ActionConfig {
            mode: "agent_cli".to_string(),
            prompt_template: Some(prompt_template.to_string()),
            prompt_inline: None,
        }
    } else if let Some(prompt_inline) = args.prompt_inline.as_deref() {
        jobs::ActionConfig {
            mode: "agent_cli".to_string(),
            prompt_template: None,
            prompt_inline: Some(prompt_inline.to_string()),
        }
    } else {
        jobs::ActionConfig {
            mode: "agent_cli".to_string(),
            prompt_template: Some("heartbeat-default".to_string()),
            prompt_inline: None,
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
}
