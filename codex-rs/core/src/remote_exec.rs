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
    pub mounts: Vec<Mount>,
    pub tmpfs: Vec<PathBuf>,
    pub limits: Option<ResourceLimits>,
    pub user: Option<String>,
}

#[derive(Clone, Debug)]
pub struct DockerSandboxExecutor {
    pub config: DockerSandboxConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mount {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub read_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    pub cpu_quota_micros: Option<u64>,
    pub memory_bytes: Option<u64>,
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

        if let Some(user) = self.config.user.as_ref() {
            if !user.trim().is_empty() {
                cmd.extend(["--user".to_string(), user.clone()]);
            }
        }

        if let Some(limits) = self.config.limits.as_ref() {
            if let Some(cpu_quota_micros) = limits.cpu_quota_micros {
                cmd.extend(["--cpu-quota".to_string(), cpu_quota_micros.to_string()]);
            }
            if let Some(memory_bytes) = limits.memory_bytes {
                cmd.extend(["--memory".to_string(), memory_bytes.to_string()]);
            }
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

        for (key, value) in request.env.iter() {
            cmd.extend(["-e".to_string(), format!("{key}={value}")]);
        }

        for mount in self.config.mounts.iter() {
            let mut spec = format!(
                "{}:{}",
                mount.host_path.to_string_lossy(),
                mount.container_path.to_string_lossy()
            );
            if mount.read_only {
                spec.push_str(":ro");
            } else {
                spec.push_str(":rw");
            }
            cmd.extend(["-v".to_string(), spec]);
        }

        for tmpfs in self.config.tmpfs.iter() {
            cmd.extend(["--tmpfs".to_string(), tmpfs.to_string_lossy().to_string()]);
        }

        for input in request.input_files.iter() {
            let host_path = input.to_string_lossy().to_string();
            let container_path = input.to_string_lossy().to_string();
            cmd.extend(["-v".to_string(), format!("{host_path}:{container_path}:ro")]);
        }

        cmd.push(self.config.image.clone());
        cmd.extend(request.command.clone());

        Ok(cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_docker_command_includes_env_and_mounts() {
        let mut env = HashMap::new();
        env.insert("LANG".to_string(), "C.UTF-8".to_string());
        let request = RemoteExecRequest {
            user_api_key_id: "key-1".to_string(),
            session_id: "sess-1".to_string(),
            command: vec!["/bin/echo".to_string(), "hello".to_string()],
            working_dir: PathBuf::from("/workspace"),
            env,
            input_files: vec![PathBuf::from("/workspace/input.txt")],
        };

        let config = DockerSandboxConfig {
            image: "codex-exec:latest".to_string(),
            workdir_in_container: PathBuf::from("/workspace"),
            readonly_root: true,
            mounts: vec![Mount {
                host_path: PathBuf::from("/data"),
                container_path: PathBuf::from("/data"),
                read_only: true,
            }],
            tmpfs: vec![PathBuf::from("/tmp")],
            limits: Some(ResourceLimits {
                cpu_quota_micros: Some(50_000),
                memory_bytes: Some(512 * 1024 * 1024),
            }),
            user: Some("1000:1000".to_string()),
        };

        let executor = DockerSandboxExecutor::new(config);
        let command = executor.build_docker_command(&request).expect("command");

        assert!(command.contains(&"--read-only".to_string()));
        assert!(command.contains(&"--network".to_string()));
        assert!(command.contains(&"none".to_string()));
        assert!(command.contains(&"-e".to_string()));
        assert!(command.contains(&"LANG=C.UTF-8".to_string()));
        assert!(command.contains(&"--cpu-quota".to_string()));
        assert!(command.contains(&"50000".to_string()));
        assert!(command.contains(&"--memory".to_string()));
        assert!(command.contains(&(512 * 1024 * 1024).to_string()));
        assert!(command.contains(&"codex-exec:latest".to_string()));
        assert!(command.contains(&"/bin/echo".to_string()));
    }

    #[test]
    fn build_docker_command_rejects_empty_image() {
        let request = RemoteExecRequest {
            user_api_key_id: "key-1".to_string(),
            session_id: "sess-1".to_string(),
            command: vec!["/bin/echo".to_string()],
            working_dir: PathBuf::from("/workspace"),
            env: HashMap::new(),
            input_files: Vec::new(),
        };
        let config = DockerSandboxConfig {
            image: " ".to_string(),
            workdir_in_container: PathBuf::from("/workspace"),
            readonly_root: false,
            mounts: Vec::new(),
            tmpfs: Vec::new(),
            limits: None,
            user: None,
        };

        let executor = DockerSandboxExecutor::new(config);
        let err = executor
            .build_docker_command(&request)
            .expect_err("empty image rejected");
        assert!(matches!(err, RemoteExecError::InvalidDockerImage(_)));
    }
}
