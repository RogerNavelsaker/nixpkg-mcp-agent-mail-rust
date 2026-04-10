use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::util::{append_line, shell_single_quote, write_string};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RunMeta {
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_seconds: Option<i64>,
    pub profile: String,
    pub profile_description: String,
    pub binary: String,
    pub project_dir: String,
    pub host: String,
    pub port: String,
    pub path: String,
    pub keys: String,
    pub seed_demo: u8,
    pub seed_required: u8,
    pub seed_exit_code: Option<i32>,
    pub snapshot_required: u8,
    pub snapshot_status: Option<String>,
    pub snapshot_exit_code: Option<i32>,
    pub vhs_exit_code: Option<i32>,
    pub host_vhs_exit_code: Option<i32>,
    pub vhs_driver_used: Option<String>,
    pub vhs_docker_log: Option<String>,
    pub video_exists: Option<bool>,
    pub snapshot_exists: Option<bool>,
    pub video_duration_seconds: Option<f64>,
    pub output: String,
    pub snapshot: String,
    pub run_dir: String,
    pub trace_id: Option<String>,
    pub fallback_active: Option<bool>,
    pub fallback_reason: Option<String>,
    pub capture_error_reason: Option<String>,
    pub ttyd_shim_log: Option<String>,
    pub ttyd_runtime_log: Option<String>,
    pub tmux_session: Option<String>,
    pub tmux_attach_command: Option<String>,
    pub tmux_session_file: Option<String>,
    pub tmux_pane_capture: Option<String>,
    pub tmux_pane_log: Option<String>,
    pub vhs_no_sandbox_forced: Option<bool>,
    pub policy_id: Option<String>,
    pub evidence_ledger: Option<String>,
    pub artifact_manifest: Option<String>,
    pub fastapi_output_mode: Option<String>,
    pub fastapi_agent_mode: Option<bool>,
    pub sqlmodel_output_mode: Option<String>,
    pub sqlmodel_agent_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    Contract,
    Media,
    Replay,
    Evidence,
    Diagnostics,
    Session,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactEntry {
    pub key: String,
    pub role: ArtifactRole,
    pub purpose: String,
    pub path: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayCommand {
    pub key: String,
    pub purpose: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunArtifactManifest {
    pub version: String,
    pub profile: String,
    pub status: String,
    pub run_dir: String,
    pub trace_id: Option<String>,
    pub artifact_count: usize,
    pub artifacts: Vec<ArtifactEntry>,
    pub replay_commands: Vec<ReplayCommand>,
}

impl RunMeta {
    pub fn write_to_path(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        write_string(path, &content)
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str::<Self>(&content)?)
    }

    fn run_dir_path(&self) -> Option<PathBuf> {
        let trimmed = self.run_dir.trim();
        (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
    }

    fn push_artifact_entry(
        entries: &mut Vec<ArtifactEntry>,
        seen_paths: &mut BTreeSet<String>,
        key: &str,
        role: ArtifactRole,
        purpose: &str,
        path: PathBuf,
    ) {
        let display = path.display().to_string();
        if display.is_empty() || !seen_paths.insert(display.clone()) {
            return;
        }

        entries.push(ArtifactEntry {
            key: key.to_string(),
            role,
            purpose: purpose.to_string(),
            exists: path.exists(),
            path: display,
        });
    }

    pub fn artifact_entries(&self) -> Vec<ArtifactEntry> {
        let Some(run_dir) = self.run_dir_path() else {
            return Vec::new();
        };

        let mut entries = Vec::new();
        let mut seen_paths = BTreeSet::new();

        Self::push_artifact_entry(
            &mut entries,
            &mut seen_paths,
            "run_meta",
            ArtifactRole::Contract,
            "Canonical per-run machine-readable metadata contract.",
            run_dir.join("run_meta.json"),
        );
        Self::push_artifact_entry(
            &mut entries,
            &mut seen_paths,
            "run_summary",
            ArtifactRole::Contract,
            "Human-readable run summary for quick triage.",
            run_dir.join("run_summary.txt"),
        );
        Self::push_artifact_entry(
            &mut entries,
            &mut seen_paths,
            "capture_tape",
            ArtifactRole::Replay,
            "Replay-grade VHS tape used to reproduce the capture sequence.",
            run_dir.join("capture.tape"),
        );

        let push_optional_path = |entries: &mut Vec<ArtifactEntry>,
                                  seen_paths: &mut BTreeSet<String>,
                                  key: &str,
                                  role: ArtifactRole,
                                  purpose: &str,
                                  value: &str| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return;
            }
            Self::push_artifact_entry(
                entries,
                seen_paths,
                key,
                role,
                purpose,
                PathBuf::from(trimmed),
            );
        };

        push_optional_path(
            &mut entries,
            &mut seen_paths,
            "capture_output",
            ArtifactRole::Media,
            "Rendered video output produced by the run.",
            &self.output,
        );
        push_optional_path(
            &mut entries,
            &mut seen_paths,
            "snapshot_image",
            ArtifactRole::Media,
            "Representative still image extracted from the capture output.",
            &self.snapshot,
        );

        if let Some(path) = &self.evidence_ledger {
            push_optional_path(
                &mut entries,
                &mut seen_paths,
                "evidence_ledger",
                ArtifactRole::Evidence,
                "Append-only decision ledger for replay and policy audits.",
                path,
            );
        }
        if let Some(path) = &self.artifact_manifest {
            push_optional_path(
                &mut entries,
                &mut seen_paths,
                "artifact_manifest",
                ArtifactRole::Contract,
                "Curated artifact inventory with replay helpers for this run.",
                path,
            );
        }
        if let Some(path) = &self.ttyd_shim_log {
            push_optional_path(
                &mut entries,
                &mut seen_paths,
                "ttyd_shim_log",
                ArtifactRole::Diagnostics,
                "Compatibility shim log for ttyd invocation adjustments.",
                path,
            );
        }
        if let Some(path) = &self.ttyd_runtime_log {
            push_optional_path(
                &mut entries,
                &mut seen_paths,
                "ttyd_runtime_log",
                ArtifactRole::Diagnostics,
                "Runtime log captured from the ttyd-backed replay server.",
                path,
            );
        }
        if let Some(path) = &self.vhs_docker_log {
            push_optional_path(
                &mut entries,
                &mut seen_paths,
                "vhs_docker_log",
                ArtifactRole::Diagnostics,
                "Docker-backed VHS execution log for fallback capture runs.",
                path,
            );
        }
        if let Some(path) = &self.tmux_session_file {
            push_optional_path(
                &mut entries,
                &mut seen_paths,
                "tmux_session_file",
                ArtifactRole::Session,
                "Session metadata for live tmux observation and attachment.",
                path,
            );
        }
        if let Some(path) = &self.tmux_pane_capture {
            push_optional_path(
                &mut entries,
                &mut seen_paths,
                "tmux_pane_capture",
                ArtifactRole::Session,
                "Snapshot of the observed tmux pane after capture completion.",
                path,
            );
        }
        if let Some(path) = &self.tmux_pane_log {
            push_optional_path(
                &mut entries,
                &mut seen_paths,
                "tmux_pane_log",
                ArtifactRole::Session,
                "Streaming log captured from the observed tmux pane.",
                path,
            );
        }

        entries
    }

    pub fn replay_commands(&self) -> Vec<ReplayCommand> {
        let Some(run_dir) = self.run_dir_path() else {
            return Vec::new();
        };

        let mut commands = Vec::new();
        let run_meta_path = run_dir.join("run_meta.json");
        let summary_path = run_dir.join("run_summary.txt");
        let tape_path = run_dir.join("capture.tape");

        commands.push(ReplayCommand {
            key: "inspect_run_meta".to_string(),
            purpose: "Open the canonical JSON contract for the run.".to_string(),
            command: format!(
                "cat {}",
                shell_single_quote(&run_meta_path.display().to_string())
            ),
        });
        commands.push(ReplayCommand {
            key: "inspect_run_summary".to_string(),
            purpose: "Read the concise text summary before deeper triage.".to_string(),
            command: format!(
                "cat {}",
                shell_single_quote(&summary_path.display().to_string())
            ),
        });

        if let Some(path) = self
            .evidence_ledger
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            commands.push(ReplayCommand {
                key: "tail_evidence_ledger".to_string(),
                purpose: "Inspect the most recent evidence-ledger decisions.".to_string(),
                command: format!("tail -n 80 {}", shell_single_quote(path)),
            });
        }

        if tape_path.exists() {
            commands.push(ReplayCommand {
                key: "replay_capture_tape".to_string(),
                purpose: "Re-run the exact VHS capture tape inside the run directory.".to_string(),
                command: format!(
                    "cd {} && vhs {}",
                    shell_single_quote(&run_dir.display().to_string()),
                    shell_single_quote("capture.tape")
                ),
            });
        }

        if let Some(command) = self
            .tmux_attach_command
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            commands.push(ReplayCommand {
                key: "attach_tmux_observer".to_string(),
                purpose: "Attach to the preserved tmux observer session for live context."
                    .to_string(),
                command: command.to_string(),
            });
        }

        commands
    }

    pub fn build_artifact_manifest(&self) -> RunArtifactManifest {
        let artifacts = self.artifact_entries();
        let replay_commands = self.replay_commands();
        RunArtifactManifest {
            version: "doctor-run-artifact-manifest-v1".to_string(),
            profile: self.profile.clone(),
            status: self.status.clone(),
            run_dir: self.run_dir.clone(),
            trace_id: self.trace_id.clone(),
            artifact_count: artifacts.len(),
            artifacts,
            replay_commands,
        }
    }

    pub fn write_artifact_manifest(&self) -> Result<Option<PathBuf>> {
        let Some(path) = self
            .artifact_manifest
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
        else {
            return Ok(None);
        };

        let mut manifest = self.build_artifact_manifest();
        if let Some(entry) = manifest
            .artifacts
            .iter_mut()
            .find(|entry| entry.key == "artifact_manifest")
        {
            entry.exists = true;
        }
        let content = serde_json::to_string_pretty(&manifest)?;
        write_string(&path, &content)?;
        Ok(Some(path))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub timestamp: String,
    pub trace_id: String,
    pub decision_id: String,
    pub action: String,
    pub evidence_terms: Vec<String>,
    pub fallback_active: bool,
    pub fallback_reason: Option<String>,
    pub policy_id: String,
}

impl DecisionRecord {
    pub fn append_jsonl(&self, path: &Path) -> Result<()> {
        let line = serde_json::to_string(self)?;
        append_line(path, &line)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{ArtifactRole, DecisionRecord, RunMeta};

    #[test]
    fn runmeta_round_trip_preserves_fields() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("run_meta.json");

        let original = RunMeta {
            status: "ok".to_string(),
            started_at: "2026-02-17T00:00:00Z".to_string(),
            finished_at: Some("2026-02-17T00:00:01Z".to_string()),
            duration_seconds: Some(1),
            profile: "analytics-empty".to_string(),
            binary: "cargo run -q -p ftui-demo-showcase".to_string(),
            output: "/tmp/out.mp4".to_string(),
            run_dir: "/tmp/run".to_string(),
            tmux_session: Some("capture-demo".to_string()),
            tmux_attach_command: Some("tmux attach-session -t capture-demo".to_string()),
            artifact_manifest: Some("/tmp/run/run_artifact_manifest.json".to_string()),
            fastapi_output_mode: Some("plain".to_string()),
            sqlmodel_output_mode: Some("json".to_string()),
            ..RunMeta::default()
        };

        original.write_to_path(&path).expect("write run_meta");
        let decoded = RunMeta::from_path(&path).expect("read run_meta");

        assert_eq!(decoded.status, original.status);
        assert_eq!(decoded.started_at, original.started_at);
        assert_eq!(decoded.finished_at, original.finished_at);
        assert_eq!(decoded.duration_seconds, original.duration_seconds);
        assert_eq!(decoded.profile, original.profile);
        assert_eq!(decoded.binary, original.binary);
        assert_eq!(decoded.output, original.output);
        assert_eq!(decoded.run_dir, original.run_dir);
        assert_eq!(decoded.tmux_session, original.tmux_session);
        assert_eq!(decoded.tmux_attach_command, original.tmux_attach_command);
        assert_eq!(decoded.artifact_manifest, original.artifact_manifest);
        assert_eq!(decoded.fastapi_output_mode, original.fastapi_output_mode);
        assert_eq!(decoded.sqlmodel_output_mode, original.sqlmodel_output_mode);
    }

    #[test]
    fn runmeta_deserialize_sparse_json_uses_defaults_for_missing_fields() {
        let sparse = r#"{"status":"failed","started_at":"2026-02-17T00:00:00Z"}"#;
        let parsed = serde_json::from_str::<RunMeta>(sparse).expect("parse sparse runmeta");

        assert_eq!(parsed.status, "failed");
        assert_eq!(parsed.started_at, "2026-02-17T00:00:00Z");
        assert_eq!(parsed.profile, "");
        assert_eq!(parsed.output, "");
        assert_eq!(parsed.seed_demo, 0);
        assert_eq!(parsed.snapshot_required, 0);
        assert!(parsed.finished_at.is_none());
    }

    #[test]
    fn decision_record_append_jsonl_writes_one_json_object_per_line() {
        let temp = tempdir().expect("tempdir");
        let ledger = temp.path().join("ledger.jsonl");

        let first = DecisionRecord {
            timestamp: "2026-02-17T00:00:00Z".to_string(),
            trace_id: "trace-1".to_string(),
            decision_id: "decision-1".to_string(),
            action: "capture_config_resolved".to_string(),
            evidence_terms: vec!["profile=analytics-empty".to_string()],
            fallback_active: false,
            fallback_reason: None,
            policy_id: "doctor_frankentui/v1".to_string(),
        };
        let second = DecisionRecord {
            timestamp: "2026-02-17T00:00:01Z".to_string(),
            trace_id: "trace-1".to_string(),
            decision_id: "decision-2".to_string(),
            action: "capture_finalize".to_string(),
            evidence_terms: vec!["final_status=ok".to_string()],
            fallback_active: true,
            fallback_reason: Some("capture timeout exceeded 30s".to_string()),
            policy_id: "doctor_frankentui/v1".to_string(),
        };

        first.append_jsonl(&ledger).expect("append first record");
        second.append_jsonl(&ledger).expect("append second record");

        let content = std::fs::read_to_string(&ledger).expect("read ledger");
        let lines = content.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);

        let parsed_first =
            serde_json::from_str::<DecisionRecord>(lines[0]).expect("parse first decision");
        let parsed_second =
            serde_json::from_str::<DecisionRecord>(lines[1]).expect("parse second decision");

        assert_eq!(parsed_first.decision_id, "decision-1");
        assert_eq!(parsed_second.decision_id, "decision-2");
        assert!(parsed_second.fallback_active);
    }

    #[test]
    fn runmeta_write_to_path_creates_parent_directories() {
        let temp = tempdir().expect("tempdir");
        let nested = temp.path().join("a/b/run_meta.json");

        let meta = RunMeta {
            status: "ok".to_string(),
            started_at: "2026-02-17T00:00:00Z".to_string(),
            profile: "analytics-empty".to_string(),
            ..RunMeta::default()
        };

        meta.write_to_path(&nested).expect("write nested run_meta");
        assert!(nested.exists());

        let decoded = RunMeta::from_path(&nested).expect("read nested run_meta");
        assert_eq!(decoded.status, meta.status);
        assert_eq!(decoded.profile, meta.profile);
    }

    #[test]
    fn artifact_entries_include_contract_and_optional_paths() {
        let temp = tempdir().expect("tempdir");
        let run_dir = temp.path().join("run");
        std::fs::create_dir_all(&run_dir).expect("run dir");
        std::fs::write(run_dir.join("run_meta.json"), "{}").expect("write run meta");
        std::fs::write(run_dir.join("run_summary.txt"), "summary").expect("write summary");
        std::fs::write(run_dir.join("capture.tape"), "Output demo").expect("write tape");
        std::fs::write(run_dir.join("capture.mp4"), b"video").expect("write video");
        std::fs::write(run_dir.join("evidence_ledger.jsonl"), b"{}\n").expect("write ledger");

        let meta = RunMeta {
            status: "ok".to_string(),
            started_at: "2026-02-17T00:00:00Z".to_string(),
            profile: "analytics-empty".to_string(),
            output: run_dir.join("capture.mp4").display().to_string(),
            run_dir: run_dir.display().to_string(),
            evidence_ledger: Some(run_dir.join("evidence_ledger.jsonl").display().to_string()),
            artifact_manifest: Some(
                run_dir
                    .join("run_artifact_manifest.json")
                    .display()
                    .to_string(),
            ),
            ..RunMeta::default()
        };

        let entries = meta.artifact_entries();
        assert!(entries.iter().any(|entry| entry.key == "run_meta"));
        assert!(entries.iter().any(|entry| entry.key == "run_summary"));
        assert!(entries.iter().any(|entry| entry.key == "capture_tape"));
        assert!(entries.iter().any(|entry| entry.key == "capture_output"));
        assert!(entries.iter().any(|entry| entry.key == "evidence_ledger"));
        assert!(entries.iter().any(|entry| entry.key == "artifact_manifest"));
        assert!(
            entries
                .iter()
                .any(|entry| entry.key == "run_meta" && entry.role == ArtifactRole::Contract)
        );
    }

    #[test]
    fn write_artifact_manifest_persists_replay_commands_and_inventory() {
        let temp = tempdir().expect("tempdir");
        let run_dir = temp.path().join("run");
        std::fs::create_dir_all(&run_dir).expect("run dir");
        std::fs::write(run_dir.join("run_meta.json"), "{}").expect("write run meta");
        std::fs::write(run_dir.join("run_summary.txt"), "summary").expect("write summary");
        std::fs::write(run_dir.join("capture.tape"), "Output demo").expect("write tape");
        std::fs::write(run_dir.join("evidence_ledger.jsonl"), b"{}\n").expect("write ledger");

        let meta = RunMeta {
            status: "failed".to_string(),
            started_at: "2026-02-17T00:00:00Z".to_string(),
            profile: "analytics-empty".to_string(),
            run_dir: run_dir.display().to_string(),
            evidence_ledger: Some(run_dir.join("evidence_ledger.jsonl").display().to_string()),
            artifact_manifest: Some(
                run_dir
                    .join("run_artifact_manifest.json")
                    .display()
                    .to_string(),
            ),
            tmux_attach_command: Some("tmux attach-session -t capture-demo".to_string()),
            ..RunMeta::default()
        };

        let manifest_path = meta
            .write_artifact_manifest()
            .expect("write artifact manifest")
            .expect("artifact manifest path");
        let manifest: super::RunArtifactManifest = serde_json::from_str(
            &std::fs::read_to_string(&manifest_path).expect("read artifact manifest"),
        )
        .expect("parse artifact manifest");

        assert_eq!(manifest.version, "doctor-run-artifact-manifest-v1");
        assert_eq!(manifest.profile, "analytics-empty");
        assert!(manifest.artifact_count >= 3);
        assert!(
            manifest
                .artifacts
                .iter()
                .any(|entry| entry.key == "artifact_manifest" && entry.exists)
        );
        assert!(
            manifest
                .replay_commands
                .iter()
                .any(|command| command.key == "replay_capture_tape")
        );
        assert!(
            manifest
                .replay_commands
                .iter()
                .any(|command| command.key == "attach_tmux_observer")
        );
    }
}
