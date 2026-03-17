//! Multi-stage pipeline executor.

use std::time::Duration;

/// Maximum bytes of stdout buffered between pipeline stages.
const MAX_STAGE_OUTPUT_BYTES: usize = 1024 * 1024; // 1 MiB

use serde::{Deserialize, Serialize};
use synwire_core::BoxFuture;
use synwire_core::sandbox::PipelineStage;
use synwire_core::vfs::error::VfsError;
use synwire_core::vfs::types::ExecuteResponse;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

/// Result of a pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// Per-stage responses.
    pub stages: Vec<ExecuteResponse>,
    /// Final combined exit code.
    pub exit_code: i32,
}

/// Executes multi-stage command pipelines.
#[derive(Debug, Default, Clone)]
pub struct PipelineExecutor;

impl PipelineExecutor {
    /// Create a new pipeline executor.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Execute a pipeline: each stage's stdout is piped to the next stage's stdin.
    pub fn execute<'a>(
        &'a self,
        stages: &'a [PipelineStage],
        default_timeout: Duration,
    ) -> BoxFuture<'a, Result<PipelineResult, VfsError>> {
        Box::pin(async move {
            if stages.is_empty() {
                return Ok(PipelineResult {
                    stages: Vec::new(),
                    exit_code: 0,
                });
            }

            let mut responses = Vec::new();
            let mut stdin_data: Option<Vec<u8>> = None;

            for stage in stages {
                let stage_timeout = stage
                    .timeout_secs
                    .map_or(default_timeout, Duration::from_secs);

                let mut cmd = Command::new(&stage.command);
                let _ = cmd
                    .args(&stage.args)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());

                let mut child = cmd.spawn().map_err(VfsError::Io)?;

                // Feed previous stage's output as stdin.
                if let Some(data) = stdin_data.take() {
                    if let Some(mut sin) = child.stdin.take() {
                        sin.write_all(&data).await.map_err(VfsError::Io)?;
                        drop(sin);
                    }
                } else {
                    drop(child.stdin.take());
                }

                let output = timeout(stage_timeout, child.wait_with_output())
                    .await
                    .map_err(|_| {
                        VfsError::Timeout(format!(
                            "{} timed out after {stage_timeout:?}",
                            stage.command
                        ))
                    })?
                    .map_err(VfsError::Io)?;

                let mut stdout = output.stdout;
                if stage.stderr_to_stdout {
                    stdout.extend_from_slice(&output.stderr);
                }
                // Cap buffered output to avoid excessive memory use between stages.
                stdout.truncate(MAX_STAGE_OUTPUT_BYTES);
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

                let resp = ExecuteResponse {
                    exit_code: output.status.code().unwrap_or(-1),
                    stdout: String::from_utf8_lossy(&stdout).into_owned(),
                    stderr,
                };

                // If stage failed, stop the pipeline.
                if resp.exit_code != 0 {
                    let exit_code = resp.exit_code;
                    responses.push(resp);
                    return Ok(PipelineResult {
                        exit_code,
                        stages: responses,
                    });
                }

                stdin_data = Some(stdout);
                responses.push(resp);
            }

            let exit_code = responses.last().map_or(0, |r| r.exit_code);

            Ok(PipelineResult {
                stages: responses,
                exit_code,
            })
        })
    }

    /// Redirect final stage stdout to a file.
    pub fn execute_to_file<'a>(
        &'a self,
        stages: &'a [PipelineStage],
        output_file: &'a str,
        default_timeout: Duration,
    ) -> BoxFuture<'a, Result<PipelineResult, VfsError>> {
        Box::pin(async move {
            let result = self.execute(stages, default_timeout).await?;
            if let Some(last) = result.stages.last() {
                tokio::fs::write(output_file, last.stdout.as_bytes())
                    .await
                    .map_err(VfsError::Io)?;
            }
            Ok(result)
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn stage(cmd: &str, args: &[&str]) -> PipelineStage {
        PipelineStage {
            command: cmd.to_string(),
            args: args.iter().map(ToString::to_string).collect(),
            stderr_to_stdout: false,
            timeout_secs: None,
        }
    }

    #[tokio::test]
    async fn test_simple_pipeline() {
        let executor = PipelineExecutor::new();
        let stages = vec![stage("echo", &["hello world"]), stage("grep", &["hello"])];
        let result = executor
            .execute(&stages, Duration::from_secs(5))
            .await
            .expect("pipeline");
        assert_eq!(result.exit_code, 0);
        assert!(result.stages[1].stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_pipeline_stage_failure_stops() {
        let executor = PipelineExecutor::new();
        let stages = vec![
            stage("false", &[]), // always fails
            stage("echo", &["should not run"]),
        ];
        let result = executor
            .execute(&stages, Duration::from_secs(5))
            .await
            .expect("pipeline");
        assert_ne!(result.exit_code, 0);
        assert_eq!(result.stages.len(), 1);
    }

    #[tokio::test]
    async fn test_stderr_to_stdout_combined() {
        let executor = PipelineExecutor::new();
        let stages = vec![PipelineStage {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "echo out; echo err >&2".to_string()],
            stderr_to_stdout: true,
            timeout_secs: None,
        }];
        let result = executor
            .execute(&stages, Duration::from_secs(5))
            .await
            .expect("pipeline");
        assert_eq!(result.exit_code, 0);
        assert!(result.stages[0].stdout.contains("out"));
    }

    #[tokio::test]
    async fn test_per_stage_timeout() {
        let executor = PipelineExecutor::new();
        let stages = vec![PipelineStage {
            command: "sleep".to_string(),
            args: vec!["10".to_string()],
            stderr_to_stdout: false,
            timeout_secs: Some(1),
        }];
        let err = executor
            .execute(&stages, Duration::from_secs(30))
            .await
            .expect_err("should timeout");
        assert!(matches!(err, VfsError::Timeout(_)));
    }
}
