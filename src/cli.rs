use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "abeat",
    version,
    about = "Local-first agentic heartbeat runner"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: AbeCommand,
}

#[derive(Debug, Subcommand)]
pub enum AbeCommand {
    /// Initialize ~/.config/abeat and ~/.abeat directories
    Init,
    /// Show resolved config/runtime paths
    Which(WhichArgs),
    /// Execute due jobs once (intended for cron/systemd)
    Tick {
        #[arg(long)]
        due: bool,
    },
    /// Run one job immediately
    Run { job_id: String },
    /// View run logs (runs.jsonl)
    Logs(LogsArgs),
    /// Convenience alias for `abeat get jobs`
    #[command(alias = "ls")]
    List(GetJobsArgs),
    /// Read resources
    Get(GetCommand),
    /// Write resources
    Set(SetCommand),
}

#[derive(Debug, Args)]
pub struct WhichArgs {
    /// Optional target: config|runtime|state|jobs|logs
    pub target: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct LogsArgs {
    #[arg(long)]
    pub job: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub limit: Option<u32>,
}

#[derive(Debug, Args, Clone)]
pub struct GetJobsArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct GetCommand {
    #[command(subcommand)]
    pub subcommand: GetSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum GetSubcommand {
    /// List job definitions
    Jobs(GetJobsArgs),
    /// Show a single job definition
    Job {
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// List runs (not yet implemented; use `abeat logs`)
    Runs,
}

#[derive(Debug, Args)]
pub struct SetCommand {
    #[command(subcommand)]
    pub subcommand: SetSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SetSubcommand {
    Jobs(SetJobsArgs),
}

#[derive(Debug, Args)]
pub struct SetJobsArgs {
    #[command(subcommand)]
    pub subcommand: SetJobsCommand,
}

#[derive(Debug, Subcommand)]
pub enum SetJobsCommand {
    Add(SetJobsAddArgs),
    Update { id: String },
    Enable { id: String },
    Disable { id: String },
    Rm { id: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum JobKindArg {
    #[value(name = "heartbeat_check")]
    HeartbeatCheck,
    #[value(name = "scheduled_task")]
    ScheduledTask,
}

#[derive(Debug, Args)]
pub struct SetJobsAddArgs {
    #[arg(long)]
    pub id: String,
    #[arg(long, value_enum)]
    pub kind: JobKindArg,
    #[arg(long)]
    pub every: Option<String>,
    #[arg(long)]
    pub cron: Option<String>,
    #[arg(long)]
    pub cooldown: Option<String>,
    #[arg(long)]
    pub agent: String,
    #[arg(long)]
    pub workspace: PathBuf,
    #[arg(long = "skill")]
    pub skill: Vec<String>,
    #[arg(long)]
    pub timeout: Option<String>,
    #[arg(long)]
    pub no_op_token: Option<String>,
    #[arg(long, default_value = "auto")]
    pub amem_mode: String,
    #[arg(long)]
    pub timezone: Option<String>,
    #[arg(long)]
    pub prompt_template: Option<String>,
    #[arg(long)]
    pub prompt_inline: Option<String>,
}
