//! The `codex workflow` subcommand group (RWO-001): the user-facing CLI
//! surface of the workflow teaching and control plane.
//!
//! The group mirrors the app-server methods through the same shared
//! [`WorkflowControlPlane`] service:
//!
//! - `codex workflow teach` runs the guided single-invocation teaching
//!   pipeline (start -> instruct/demonstrate -> reconcile -> compile ->
//!   review -> approve -> publish). Teaching state is deliberately
//!   process-local (the RWO-001 in-memory-double bound); the durable
//!   artifacts are the immutable versions publication seals.
//! - `codex workflow instance ...` operates the durable instance
//!   lifecycle (run, list, get, resume, cancel) across invocations.

use std::io::IsTerminal;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use clap::Parser;
use clap::ValueEnum;
use codex_app_server::WorkflowControlPlane;
use codex_app_server_protocol as rpc;
use codex_core::config::find_codex_home;
use codex_git_utils::get_head_commit_hash;

/// The `codex workflow` command group.
#[derive(Debug, Parser)]
pub struct WorkflowCli {
    #[command(subcommand)]
    pub subcommand: WorkflowSubcommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum WorkflowSubcommand {
    /// Teach a workflow, compile it, review it, approve it, and publish
    /// an immutable version — one guided invocation.
    Teach(TeachArgs),
    /// Operate durable workflow instances.
    Instance(InstanceCommand),
}

/// How a workflow is taught.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TeachModeArg {
    /// The user demonstrates the workflow by performing it.
    Demonstrate,
    /// The user describes the workflow in instruction statements.
    Instruct,
    /// Demonstrated steps and instructed steps are interleaved.
    Hybrid,
}

impl TeachModeArg {
    fn to_protocol(self) -> rpc::WorkflowTeachMode {
        match self {
            Self::Demonstrate => rpc::WorkflowTeachMode::Demonstrate,
            Self::Instruct => rpc::WorkflowTeachMode::Instruct,
            Self::Hybrid => rpc::WorkflowTeachMode::Hybrid,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Demonstrate => "demonstrate",
            Self::Instruct => "instruct",
            Self::Hybrid => "hybrid",
        }
    }
}

#[derive(Debug, Parser)]
pub struct TeachArgs {
    /// How the workflow is taught.
    #[arg(long = "mode", value_enum, default_value_t = TeachModeArg::Hybrid)]
    pub mode: TeachModeArg,

    /// Name of the workflow; becomes its definition id.
    #[arg(long = "name", value_name = "NAME")]
    pub name: Option<String>,

    /// Record the given instruction statement. Repeatable.
    #[arg(long = "instruct", value_name = "TEXT")]
    pub instructions: Vec<String>,

    /// Record the given demonstrated action. Repeatable.
    #[arg(long = "demonstrate", value_name = "TEXT")]
    pub demonstrations: Vec<String>,

    /// Canonical, credential-free repository identity to publish from.
    #[arg(long = "repo", value_name = "REMOTE")]
    pub repository: Option<String>,

    /// Full commit SHA (40 or 64 lowercase hex) anchoring the version.
    /// Defaults to the current repository's HEAD commit.
    #[arg(long = "commit-sha", value_name = "SHA")]
    pub commit_sha: Option<String>,

    /// Semantic version of this revision.
    #[arg(long = "version", value_name = "SEMVER")]
    pub semantic_version: Option<String>,

    /// Approver label recorded with the approval.
    #[arg(long = "approver", value_name = "LABEL", default_value = "cli-user")]
    pub approver: String,

    /// Approve without prompting (non-interactive publishing).
    #[arg(long = "yes", default_value_t = false)]
    pub yes: bool,

    /// Output every stage's full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct InstanceCommand {
    #[command(subcommand)]
    pub subcommand: InstanceSubcommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum InstanceSubcommand {
    /// Run one instance of a published workflow version.
    Run(InstanceRunArgs),
    /// List every durable instance with status and position.
    List(InstanceListArgs),
    /// Read one durable instance with its evidence references.
    Get(InstanceGetArgs),
    /// Resume one paused instance.
    Resume(InstanceResumeArgs),
    /// Cancel one instance with an explicit reason.
    Cancel(InstanceCancelArgs),
}

#[derive(Debug, Parser)]
pub struct InstanceRunArgs {
    /// The published workflow version to run, in sha256-hex form.
    pub version_id: String,

    /// Output the full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct InstanceListArgs {
    /// Output the full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct InstanceGetArgs {
    /// The durable instance to read.
    pub instance_id: String,

    /// Output the full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct InstanceResumeArgs {
    /// The paused instance to resume.
    pub instance_id: String,

    /// Output the full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct InstanceCancelArgs {
    /// The instance to cancel.
    pub instance_id: String,

    /// Operator-visible reason recorded with the cancellation.
    pub reason: String,

    /// Output the full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
}

/// The durable control-plane root under the Codex home.
fn control_plane_root() -> Result<PathBuf> {
    Ok(find_codex_home()?.join("workflow"))
}

/// Entry point for `codex workflow ...`.
pub async fn run(cli: WorkflowCli) -> Result<()> {
    let plane = WorkflowControlPlane::new(control_plane_root()?);
    match cli.subcommand {
        WorkflowSubcommand::Teach(args) => run_teach(&plane, args).await,
        WorkflowSubcommand::Instance(command) => run_instance(&plane, command).await,
    }
}

/// Runs the guided teaching pipeline.
async fn run_teach(plane: &WorkflowControlPlane, args: TeachArgs) -> Result<()> {
    let (instructions, demonstrations) =
        read_teaching_input(&args).context("read teaching input")?;
    let start = plane
        .teach_start(rpc::WorkflowTeachStartParams {
            mode: args.mode.to_protocol(),
            name: args.name.clone(),
        })
        .map_err(command_error)?;
    print_stage(
        args.json,
        "teach/start",
        &serde_json::to_value(&start)?,
        || {
            println!(
                "Teaching session {} opened (mode {}, workflow `{}`).",
                start.session_id,
                args.mode.label(),
                start.name
            );
        },
    );

    for text in instructions {
        let recorded = plane
            .teach_instruct(rpc::WorkflowTeachInstructParams {
                session_id: start.session_id.clone(),
                text,
                evidence: Vec::new(),
            })
            .map_err(command_error)?;
        print_stage(args.json, "teach/instruct", &serde_json::to_value(&recorded)?, || {});
    }
    for text in demonstrations {
        let recorded = plane
            .teach_demonstrate(rpc::WorkflowTeachDemonstrateParams {
                session_id: start.session_id.clone(),
                kind: rpc::WorkflowDemonstrationKind::Action,
                text,
                evidence: Vec::new(),
            })
            .map_err(command_error)?;
        print_stage(args.json, "teach/demonstrate", &serde_json::to_value(&recorded)?, || {});
    }

    let reconciled = plane
        .teach_reconcile(rpc::WorkflowTeachReconcileParams {
            session_id: start.session_id.clone(),
        })
        .map_err(command_error)?;
    print_stage(
        args.json,
        "teach/reconcile",
        &serde_json::to_value(&reconciled)?,
        || {
            println!(
                "Trajectory reconciled: {} instruction(s), {} demonstration(s).",
                reconciled.instruction_records, reconciled.demonstration_records
            );
        },
    );

    let compiled = plane
        .compile(rpc::WorkflowCompileParams {
            session_id: start.session_id.clone(),
        })
        .map_err(command_error)?;
    print_stage(
        args.json,
        "compile",
        &serde_json::to_value(&compiled)?,
        || {
            println!(
                "Compiled {}: {} step(s), status {}.",
                compiled.candidate_id,
                compiled.step_count,
                candidate_status_label(&compiled.status)
            );
        },
    );

    let review = plane
        .review(rpc::WorkflowReviewParams {
            candidate_id: compiled.candidate_id.clone(),
        })
        .map_err(command_error)?;
    print_stage(args.json, "review", &serde_json::to_value(&review)?, || {
        println!("Review of {}:", review.candidate_id);
        for step in &review.steps {
            println!(
                "  {} [{}] {}",
                step.node_id,
                step_origin_label(&step.origin),
                step.description.as_deref().unwrap_or("(no description)")
            );
        }
        for proposal in &review.binding_proposals {
            println!(
                "  {} binding proposal: {} ({})",
                proposal.node_id, proposal.reference, proposal.rationale
            );
        }
        for intent in &review.trigger_intents {
            println!("  trigger intent: {}", intent.description);
        }
        if !review.validation.findings.is_empty() {
            println!("  validation findings:");
            for finding in &review.validation.findings {
                println!(
                    "    [{}] {}: {}",
                    severity_label(&finding.severity),
                    finding.code,
                    finding.message
                );
            }
        }
    });

    if !args.yes {
        if std::io::stdin().is_terminal() {
            print!("Approve and publish this workflow? [y/N] ");
            std::io::stdout().flush()?;
            let mut answer = String::new();
            std::io::stdin().read_line(&mut answer)?;
            if !answer.trim().eq_ignore_ascii_case("y") {
                bail!("teaching aborted before approval; nothing was published");
            }
        } else {
            bail!(
                "refusing to publish without approval in non-interactive mode; pass --yes to approve"
            );
        }
    }

    let approved = plane
        .approve(rpc::WorkflowApproveParams {
            candidate_id: compiled.candidate_id.clone(),
            approver: args.approver.clone(),
            reference: format!("cli-teach-{}", start.session_id),
            decision: rpc::WorkflowApprovalDecision::Approved,
        })
        .map_err(command_error)?;
    print_stage(
        args.json,
        "approve",
        &serde_json::to_value(&approved)?,
        || {
            println!(
                "Candidate {} approved by {}.",
                approved.candidate_id, approved.approver
            );
        },
    );

    let commit_sha = match args.commit_sha {
        Some(sha) => sha,
        None => {
            let cwd = std::env::current_dir()?;
            get_head_commit_hash(&cwd)
                .await
                .with_context(|| "resolve the current repository's HEAD commit")?
                .0
        }
    };
    let published = plane
        .publish(rpc::WorkflowPublishParams {
            candidate_id: compiled.candidate_id.clone(),
            repository: args.repository.clone(),
            commit_sha,
            semantic_version: args.semantic_version.clone(),
        })
        .map_err(command_error)?;
    print_stage(args.json, "publish", &serde_json::to_value(&published)?, || {
        println!("Published {}:", published.workflow);
        println!("  version:          {}", published.semantic_version);
        println!("  versionId:        {}", published.version_id);
        println!("  definitionDigest: {}", published.definition_digest);
        println!("  dependencyLock:   {}", published.dependency_lock_digest);
        println!("  repository:       {}", published.repository);
        println!("  commit:           {}", published.commit_sha);
        println!(
            "Run it with: codex workflow instance run {}",
            published.version_id
        );
    });
    Ok(())
}

/// Collects the instruction and demonstration statements for a teach run.
fn read_teaching_input(args: &TeachArgs) -> Result<(Vec<String>, Vec<String>)> {
    if !args.instructions.is_empty() || !args.demonstrations.is_empty() {
        return Ok((args.instructions.clone(), args.demonstrations.clone()));
    }
    if std::io::stdin().is_terminal() {
        eprintln!("Teaching mode {}. Enter statements:", args.mode.label());
        eprintln!("  i <text>   — one instruction statement");
        eprintln!("  d <text>   — one demonstrated action");
        eprintln!("End with an empty line (or EOF).");
    }
    let mut instructions = Vec::new();
    let mut demonstrations = Vec::new();
    let stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        line.clear();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(text) = trimmed.strip_prefix("i ").or_else(|| trimmed.strip_prefix("i:")) {
            instructions.push(text.trim().to_string());
        } else if let Some(text) = trimmed.strip_prefix("d ").or_else(|| trimmed.strip_prefix("d:"))
        {
            demonstrations.push(text.trim().to_string());
        } else {
            bail!("unrecognized teaching line `{trimmed}`; use `i <text>` or `d <text>`");
        }
    }
    if instructions.is_empty() && demonstrations.is_empty() {
        bail!("no teaching statements were provided; nothing to compile");
    }
    Ok((instructions, demonstrations))
}

/// Entry point for `codex workflow instance ...`.
async fn run_instance(plane: &WorkflowControlPlane, command: InstanceCommand) -> Result<()> {
    match command.subcommand {
        InstanceSubcommand::Run(args) => {
            let response = plane
                .instance_run(rpc::WorkflowInstanceRunParams {
                    version_id: args.version_id,
                })
                .await
                .map_err(command_error)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                println!(
                    "instance {} {} ({})",
                    response.instance_id,
                    instance_status_label(&response.status),
                    response.workflow
                );
                println!("  version:  {}", response.version_id);
                println!("  terminal: {}", terminal_kind_label(&response.terminal.kind));
                if let Some(node) = response.terminal.node.as_deref() {
                    println!("  at node:  {node}");
                }
                if let Some(reason) = response.terminal.reason.as_deref() {
                    println!("  reason:   {reason}");
                }
            }
        }
        InstanceSubcommand::List(args) => {
            let response = plane
                .instance_list(rpc::WorkflowInstanceListParams {})
                .map_err(command_error)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else if response.instances.is_empty() {
                println!("no workflow instances");
            } else {
                for instance in &response.instances {
                    println!(
                        "{}  {}  {}",
                        instance.instance_id,
                        instance_status_label(&instance.status),
                        instance.workflow
                    );
                    println!("  version:  {}", instance.version_id);
                    if let Some(position) = &instance.position {
                        println!(
                            "  position: node {} ({} step(s) taken)",
                            position
                                .current_node
                                .as_deref()
                                .unwrap_or("(continuation frames)"),
                            position.steps_taken
                        );
                    }
                }
            }
        }
        InstanceSubcommand::Get(args) => {
            let response = plane
                .instance_get(rpc::WorkflowInstanceGetParams {
                    instance_id: args.instance_id,
                })
                .map_err(command_error)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                let instance = &response.instance;
                println!(
                    "{}  {}  {}",
                    instance.instance_id,
                    instance_status_label(&instance.status),
                    instance.workflow
                );
                println!("  version:  {}", instance.version_id);
                for evidence in &response.evidence {
                    println!(
                        "  evidence: {} {} ({})",
                        evidence.kind, evidence.locator, evidence.digest
                    );
                }
            }
        }
        InstanceSubcommand::Resume(args) => {
            let response = plane
                .instance_resume(rpc::WorkflowInstanceResumeParams {
                    instance_id: args.instance_id,
                })
                .await
                .map_err(command_error)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                println!(
                    "instance {} {}",
                    response.instance.instance_id,
                    instance_status_label(&response.instance.status)
                );
            }
        }
        InstanceSubcommand::Cancel(args) => {
            let response = plane
                .instance_cancel(rpc::WorkflowInstanceCancelParams {
                    instance_id: args.instance_id,
                    reason: args.reason,
                })
                .map_err(command_error)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                println!(
                    "instance {} {}",
                    response.instance.instance_id,
                    instance_status_label(&response.instance.status)
                );
            }
        }
    }
    Ok(())
}

/// Prints one stage: JSON when requested, a summary line otherwise.
fn print_stage(json: bool, stage: &str, value: &serde_json::Value, summary: impl FnOnce()) {
    if json {
        println!("{{\"stage\": \"{stage}\", \"response\": {value}}}");
    } else {
        summary();
    }
}

/// Maps a control-plane error to a CLI error.
fn command_error(error: codex_app_server::WorkflowControlPlaneError) -> anyhow::Error {
    anyhow::Error::msg(error.to_string())
}

/// The lowercase label of a candidate status.
fn candidate_status_label(status: &rpc::WorkflowCandidateStatus) -> &'static str {
    match status {
        rpc::WorkflowCandidateStatus::Compiled => "compiled",
        rpc::WorkflowCandidateStatus::Validated => "validated",
        rpc::WorkflowCandidateStatus::Approved => "approved",
        rpc::WorkflowCandidateStatus::PublicationReady => "publication-ready",
    }
}

/// The lowercase label of an instance status.
fn instance_status_label(status: &rpc::WorkflowInstanceStatus) -> &'static str {
    match status {
        rpc::WorkflowInstanceStatus::Pending => "pending",
        rpc::WorkflowInstanceStatus::Running => "running",
        rpc::WorkflowInstanceStatus::Paused => "paused",
        rpc::WorkflowInstanceStatus::Succeeded => "succeeded",
        rpc::WorkflowInstanceStatus::Failed => "failed",
        rpc::WorkflowInstanceStatus::Cancelled => "cancelled",
    }
}

/// The lowercase label of a run terminal kind.
fn terminal_kind_label(kind: &rpc::WorkflowRunTerminalKind) -> &'static str {
    match kind {
        rpc::WorkflowRunTerminalKind::Completed => "completed",
        rpc::WorkflowRunTerminalKind::Paused => "paused",
        rpc::WorkflowRunTerminalKind::Failed => "failed",
    }
}

/// The lowercase label of a step origin.
fn step_origin_label(origin: &rpc::WorkflowStepOrigin) -> &'static str {
    match origin {
        rpc::WorkflowStepOrigin::Observed => "observed",
        rpc::WorkflowStepOrigin::Instructed => "instructed",
    }
}

/// The lowercase label of a validation severity.
fn severity_label(severity: &rpc::WorkflowValidationSeverity) -> &'static str {
    match severity {
        rpc::WorkflowValidationSeverity::Error => "error",
        rpc::WorkflowValidationSeverity::Warning => "warning",
    }
}
