use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ExecArtifact {
    pub path: PathBuf,
    pub content_type: Option<String>,
    pub size_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct ExecOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u128,
}

#[derive(Clone, Debug)]
pub struct ExecRunResult {
    pub output: ExecOutput,
    pub artifacts: Vec<ExecArtifact>,
}

#[derive(Clone, Debug)]
pub struct RemoteExecRequest {
    pub user_api_key_id: String,
    pub session_id: String,
    pub command: Vec<String>,
    pub working_dir: PathBuf,
    pub env: HashMap<String, String>,
    pub input_files: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct OssLocation {
    pub bucket: String,
    pub key: String,
}

#[derive(thiserror::Error, Debug)]
pub enum RemoteExecError {
    #[error("invalid docker image: {0}")]
    InvalidDockerImage(String),
    #[error("unsupported command: {0}")]
    UnsupportedCommand(String),
}

pub trait ArtifactStore {
    fn store_file(&self, artifact: &ExecArtifact) -> Result<OssLocation, RemoteExecError>;
    fn store_exec_output(&self, output: &ExecOutput) -> Result<OssLocation, RemoteExecError>;
}

#[derive(Clone, Debug)]
pub struct DockerSandboxConfig {
    pub image: String,
    pub workdir_in_container: PathBuf,
    pub readonly_root: bool,
}

#[derive(Clone, Debug)]
pub struct DockerSandboxExecutor {
    pub config: DockerSandboxConfig,
}

impl DockerSandboxExecutor {
    pub fn new(config: DockerSandboxConfig) -> Self {
        Self { config }
    }

    pub fn build_docker_command(
        &self,
        request: &RemoteExecRequest,
    ) -> Result<Vec<String>, RemoteExecError> {
        if self.config.image.trim().is_empty() {
            return Err(RemoteExecError::InvalidDockerImage(
                "image name cannot be empty".to_string(),
            ));
        }
        if request.command.is_empty() {
            return Err(RemoteExecError::UnsupportedCommand(
                "command cannot be empty".to_string(),
            ));
        }

        let mut cmd = vec![
            "docker".to_string(),
            "run".to_string(),
            "--rm".to_string(),
            "--network".to_string(),
            "none".to_string(),
        ];

        if self.config.readonly_root {
            cmd.push("--read-only".to_string());
        }

        let workdir = self
            .config
            .workdir_in_container
            .to_string_lossy()
            .to_string();
        cmd.extend(["--workdir".to_string(), workdir]);

        let host_workdir = request.working_dir.to_string_lossy().to_string();
        let container_workdir = self
            .config
            .workdir_in_container
            .to_string_lossy()
            .to_string();
        cmd.extend([
            "-v".to_string(),
            format!("{host_workdir}:{container_workdir}:rw"),
        ]);

        cmd.push(self.config.image.clone());
        cmd.extend(request.command.clone());

        Ok(cmd)
    }
}
