use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

#[derive(Debug, Clone)]
pub struct AbeatPaths {
    pub config_root: PathBuf,
    pub runtime_root: PathBuf,
}

impl AbeatPaths {
    pub fn jobs_dir(&self) -> PathBuf {
        self.config_root.join("jobs")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.runtime_root.join("logs")
    }

    pub fn state_file(&self) -> PathBuf {
        self.runtime_root.join("state/jobs-state.json")
    }

    pub fn runs_file(&self) -> PathBuf {
        self.runtime_root.join("logs/runs.jsonl")
    }

    pub fn stdout_logs_dir(&self) -> PathBuf {
        self.runtime_root.join("logs/stdout")
    }

    pub fn stderr_logs_dir(&self) -> PathBuf {
        self.runtime_root.join("logs/stderr")
    }

    pub fn locks_dir(&self) -> PathBuf {
        self.runtime_root.join("state/locks")
    }

    pub fn config_dirs(&self) -> Vec<PathBuf> {
        vec![
            self.config_root.clone(),
            self.config_root.join("jobs"),
            self.config_root.join("prompts"),
            self.config_root.join("adapters"),
            self.config_root.join("context"),
            self.config_root.join("scripts"),
        ]
    }

    pub fn runtime_dirs(&self) -> Vec<PathBuf> {
        vec![
            self.runtime_root.clone(),
            self.runtime_root.join("state"),
            self.runtime_root.join("state/locks"),
            self.runtime_root.join("logs"),
            self.runtime_root.join("logs/stdout"),
            self.runtime_root.join("logs/stderr"),
            self.runtime_root.join("cache"),
            self.runtime_root.join("cache/contexts"),
        ]
    }
}

pub fn resolve_paths() -> Result<AbeatPaths> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")?;

    let config_root = if let Some(v) = env::var_os("ABEAT_CONFIG_DIR") {
        PathBuf::from(v)
    } else if let Some(v) = env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(v).join("abeat")
    } else {
        home.join(".config/abeat")
    };

    let runtime_root = if let Some(v) = env::var_os("ABEAT_DIR") {
        PathBuf::from(v)
    } else {
        home.join(".abeat")
    };

    if config_root.as_os_str().is_empty() || runtime_root.as_os_str().is_empty() {
        bail!("resolved abeat paths must not be empty");
    }

    Ok(AbeatPaths {
        config_root,
        runtime_root,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_and_runtime_subdirs_are_stable() {
        let paths = AbeatPaths {
            config_root: PathBuf::from("/tmp/config/abeat"),
            runtime_root: PathBuf::from("/tmp/.abeat"),
        };

        assert_eq!(paths.jobs_dir(), PathBuf::from("/tmp/config/abeat/jobs"));
        assert_eq!(paths.logs_dir(), PathBuf::from("/tmp/.abeat/logs"));
        assert!(
            paths
                .config_dirs()
                .contains(&PathBuf::from("/tmp/config/abeat/adapters"))
        );
        assert!(
            paths
                .runtime_dirs()
                .contains(&PathBuf::from("/tmp/.abeat/state/locks"))
        );
    }
}
