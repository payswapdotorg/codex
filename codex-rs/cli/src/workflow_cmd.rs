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
//! - `codex workflow fork` forks a published version into a new immutable
//!   release whose lineage pins the upstream (RWO-005): forked-from
//!   version id, upstream digests, and carried attribution render on the
//!   fork and are inspectable with `--json`.
//! - `codex workflow improve ...` runs the governed improvement
//!   lifecycle (RWO-006): propose candidates from recorded execution
//!   evidence, validate them (replay, differential, policy), approve or
//!   reject explicitly — the gate publication refuses to cross without —
//!   then publish the approved candidate as a new immutable successor
//!   version whose lineage pins the full decision trail. Improvement
//!   sessions are process-local (the RWO-001 in-memory-double bound),
//!   exactly like teaching; the durable artifacts are the immutable
//!   successor versions and the durable evidence references.
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
    /// Fork a published workflow version into a new immutable release
    /// that carries its lineage.
    Fork(ForkArgs),
    /// Run the governed improvement lifecycle: propose from evidence,
    /// validate, approve, publish.
    Improve(ImproveCommand),
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

/// Arguments of `codex workflow fork` (RWO-005).
#[derive(Debug, Parser)]
pub struct ForkArgs {
    /// The published version to fork, as <workflow>@<version-id>. The
    /// version id (sha256:...) is the authoritative release identity.
    #[arg(value_name = "WORKFLOW@VERSION_ID")]
    pub target: String,

    /// The fork's own repository identity; must differ from the
    /// upstream's.
    #[arg(long = "as", value_name = "REMOTE")]
    pub as_repository: String,

    /// Semantic version of the fork release; defaults to the upstream's.
    #[arg(long = "version", value_name = "SEMVER")]
    pub semantic_version: Option<String>,

    /// Full commit SHA (40 or 64 lowercase hex) the fork stands at;
    /// defaults to the upstream's.
    #[arg(long = "commit-sha", value_name = "SHA")]
    pub commit_sha: Option<String>,

    /// The owning principal of the fork.
    #[arg(long = "owner", value_name = "PRINCIPAL", default_value = "cli-user")]
    pub owner: String,

    /// SPDX-style license identifier recorded on the fork release.
    #[arg(
        long = "license",
        value_name = "SPDX",
        default_value = "LicenseRef-Unspecified"
    )]
    pub license: String,

    /// Attribution carried forward from the upstream: `Name` or
    /// `Name <contact>`. Repeatable; a fork must carry at least one
    /// (the engine refuses an empty carried attribution).
    #[arg(long = "carry", value_name = "ATTRIBUTION")]
    pub carry: Vec<String>,

    /// Output the full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
}

/// The `codex workflow improve` command group (RWO-006).
#[derive(Debug, Parser)]
pub struct ImproveCommand {
    #[command(subcommand)]
    pub subcommand: ImproveSubcommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum ImproveSubcommand {
    /// Propose improvement candidates for a published version from newly
    /// recorded execution evidence.
    Propose(ImproveProposeArgs),
    /// Validate one candidate through the replay, differential, and
    /// policy gates.
    Validate(ImproveValidateArgs),
    /// Approve a validated candidate: the gate publication requires.
    Approve(ImproveApproveArgs),
    /// Reject a validated candidate with an explicit reason.
    Reject(ImproveRejectArgs),
    /// Publish an approved candidate as a new immutable successor
    /// version.
    Publish(ImprovePublishArgs),
}

#[derive(Debug, Parser)]
pub struct ImproveProposeArgs {
    /// The published workflow version to improve, in sha256-hex form.
    pub version_id: String,

    /// Output the full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct ImproveValidateArgs {
    /// The improvement candidate to validate.
    pub candidate_id: String,

    /// The semantic version of the proposed successor (an allowed patch
    /// or minor bump of the incumbent).
    #[arg(long = "version", value_name = "SEMVER")]
    pub successor_version: String,

    /// Output the full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct ImproveApproveArgs {
    /// The validated candidate to approve.
    pub candidate_id: String,

    /// The approving principal (human or policy identity).
    #[arg(
        long = "approver",
        value_name = "PRINCIPAL",
        default_value = "cli-user"
    )]
    pub approver: String,

    /// Optional note recorded with the approval.
    #[arg(long = "note", value_name = "TEXT")]
    pub note: Option<String>,

    /// Output the full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct ImproveRejectArgs {
    /// The validated candidate to reject.
    pub candidate_id: String,

    /// The rejecting principal (human or policy identity).
    #[arg(
        long = "approver",
        value_name = "PRINCIPAL",
        default_value = "cli-user"
    )]
    pub approver: String,

    /// Why the candidate is rejected (recorded with the decision).
    #[arg(long = "reason", value_name = "TEXT")]
    pub reason: String,

    /// Output the full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct ImprovePublishArgs {
    /// The validated and approved candidate to publish.
    pub candidate_id: String,

    /// The release tag the successor is published under.
    #[arg(long = "tag", value_name = "TAG")]
    pub release_tag: String,

    /// Output the full response as JSON.
    #[arg(long = "json", default_value_t = false)]
    pub json: bool,
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
    Ok(find_codex_home()?.join("workflow").as_path().to_path_buf())
}

/// Entry point for `codex workflow ...`.
pub async fn run(cli: WorkflowCli) -> Result<()> {
    let plane = WorkflowControlPlane::new(control_plane_root()?);
    match cli.subcommand {
        WorkflowSubcommand::Teach(args) => run_teach(&plane, args).await,
        WorkflowSubcommand::Fork(args) => run_fork(&plane, args),
        WorkflowSubcommand::Improve(command) => run_improve(&plane, command).await,
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
        print_stage(
            args.json,
            "teach/instruct",
            &serde_json::to_value(&recorded)?,
            || {},
        );
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
        print_stage(
            args.json,
            "teach/demonstrate",
            &serde_json::to_value(&recorded)?,
            || {},
        );
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
    print_stage(
        args.json,
        "publish",
        &serde_json::to_value(&published)?,
        || {
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
        },
    );
    Ok(())
}

/// Runs the fork command: one published version in, one new immutable
/// release out, with the upstream pinned in its lineage.
fn run_fork(plane: &WorkflowControlPlane, args: ForkArgs) -> Result<()> {
    let (_workflow, version_id) = args
        .target
        .rsplit_once('@')
        .filter(|(workflow, version_id)| !workflow.is_empty() && !version_id.is_empty())
        .with_context(|| {
            format!(
                "`{}` is not a <workflow>@<version-id> release reference",
                args.target
            )
        })?;
    let attribution = args
        .carry
        .iter()
        .map(|value| parse_attribution(value))
        .collect::<Result<Vec<_>>>()?;
    if attribution.is_empty() {
        bail!("a fork must carry upstream attribution; pass --carry <NAME> at least once");
    }
    let response = plane
        .fork(rpc::WorkflowForkParams {
            version_id: version_id.to_string(),
            fork_repository: args.as_repository.clone(),
            semantic_version: args.semantic_version.clone(),
            commit_sha: args.commit_sha.clone(),
            owner: Some(args.owner.clone()),
            license: Some(args.license.clone()),
            attribution,
        })
        .map_err(command_error)?;
    // The version digest is the authoritative release identity: the
    // response names the workflow actually forked (rendered below in
    // both output modes), so a mislabeled <workflow>@ prefix is visible
    // rather than silently trusted.
    if args.json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Forked {} as a new immutable release:", response.workflow);
        println!("  version:          {}", response.semantic_version);
        println!("  versionId:        {}", response.version_id);
        println!("  definitionDigest: {}", response.definition_digest);
        println!("  dependencyLock:   {}", response.dependency_lock_digest);
        println!("  repository:       {}", response.repository);
        println!("  commit:           {}", response.commit_sha);
        println!(
            "  forked from:      {} {} ({})",
            response.lineage.workflow,
            response.lineage.semantic_version,
            response.lineage.version_id
        );
        println!(
            "    upstream digest: {}",
            response.lineage.definition_digest
        );
        println!(
            "    upstream lock:   {}",
            response.lineage.dependency_lock_digest
        );
        println!("    upstream commit: {}", response.lineage.commit_sha);
        println!("    upstream repo:   {}", response.lineage.repository);
        println!("  carried attribution:");
        for entry in &response.attribution {
            match entry.contact.as_deref() {
                Some(contact) => println!("    {} <{}>", entry.name, contact),
                None => println!("    {}", entry.name),
            }
        }
        println!(
            "Run it with: codex workflow instance run {}",
            response.version_id
        );
    }
    Ok(())
}

/// Entry point for `codex workflow improve ...`.
async fn run_improve(plane: &WorkflowControlPlane, command: ImproveCommand) -> Result<()> {
    match command.subcommand {
        ImproveSubcommand::Propose(args) => {
            let response = plane
                .improve_propose(rpc::WorkflowImproveProposeParams {
                    version_id: args.version_id,
                })
                .await
                .map_err(command_error)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                println!(
                    "Recorded execution evidence for {} {} ({}):",
                    response.workflow,
                    response.incumbent_semantic_version,
                    response.incumbent_version_id
                );
                print_evidence_summary(&response.evidence);
                if response.candidates.is_empty() {
                    println!("The recorded evidence supports no improvement candidate.");
                } else {
                    println!("Improvement candidates (nothing validated or approved yet):");
                    for candidate in &response.candidates {
                        println!(
                            "  {}  [{}]",
                            candidate.candidate_id,
                            change_kind_label(&candidate.change_kind)
                        );
                        println!("    {}", candidate.rationale);
                        println!(
                            "    cites {} evidence reference(s), {} run(s)",
                            candidate.evidence.references.len(),
                            candidate.evidence.runs.len()
                        );
                    }
                }
            }
        }
        ImproveSubcommand::Validate(args) => {
            let response = plane
                .improve_validate(rpc::WorkflowImproveValidateParams {
                    candidate_id: args.candidate_id,
                    successor_version: args.successor_version,
                })
                .await
                .map_err(command_error)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                println!(
                    "candidate {} ({}) — validation {}:",
                    response.candidate_id,
                    response.workflow,
                    if response.passed { "PASSED" } else { "FAILED" }
                );
                for stage in &response.stages {
                    println!(
                        "  {}: {}",
                        validation_stage_label(&stage.stage),
                        if stage.passed { "passed" } else { "FAILED" }
                    );
                }
            }
        }
        ImproveSubcommand::Approve(args) => {
            let response = plane
                .improve_approve(rpc::WorkflowImproveApproveParams {
                    candidate_id: args.candidate_id,
                    approver: args.approver,
                    decision: rpc::WorkflowImprovementDecision::Approved,
                    note: args.note,
                    reason: None,
                })
                .await
                .map_err(command_error)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                print_approval(&response);
            }
        }
        ImproveSubcommand::Reject(args) => {
            let response = plane
                .improve_approve(rpc::WorkflowImproveApproveParams {
                    candidate_id: args.candidate_id,
                    approver: args.approver,
                    decision: rpc::WorkflowImprovementDecision::Rejected,
                    note: None,
                    reason: Some(args.reason),
                })
                .await
                .map_err(command_error)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                print_approval(&response);
            }
        }
        ImproveSubcommand::Publish(args) => {
            let response = plane
                .improve_publish(rpc::WorkflowImprovePublishParams {
                    candidate_id: args.candidate_id,
                    release_tag: args.release_tag,
                })
                .await
                .map_err(command_error)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                println!(
                    "Published {} {} as a new immutable version:",
                    response.workflow, response.semantic_version
                );
                println!("  version:          {}", response.version_id);
                println!("  definitionDigest: {}", response.definition_digest);
                println!("  dependencyLock:   {}", response.dependency_lock_digest);
                println!("  repository:       {}", response.repository);
                println!("  commit:           {}", response.commit_sha);
                let lineage = &response.lineage;
                println!(
                    "  evolved from:     {} {} ({})",
                    lineage.workflow,
                    lineage.predecessor_semantic_version,
                    lineage.predecessor_version_id
                );
                println!("  candidate:        {}", lineage.candidate_id);
                println!("  validation:       {}", lineage.validation_digest);
                for stage in &lineage.validation_stages {
                    println!(
                        "    {}: {}",
                        validation_stage_label(&stage.stage),
                        if stage.passed { "passed" } else { "FAILED" }
                    );
                }
                println!("  approved by:      {}", lineage.approver);
                println!("  release tag:      {}", lineage.release_tag);
                println!("  evidence cited:");
                print_evidence_summary(&response.evidence);
                println!(
                    "Run it with: codex workflow instance run {}",
                    response.version_id
                );
            }
        }
    }
    Ok(())
}

/// Prints the evidence summary: references plus run provenance.
fn print_evidence_summary(evidence: &rpc::WorkflowEvidenceSummary) {
    for reference in &evidence.references {
        println!(
            "    {} {} ({})",
            reference.kind, reference.locator, reference.digest
        );
    }
    for run in &evidence.runs {
        println!(
            "    run of {} — {} ({})",
            run.version_id,
            run.fingerprint,
            instance_status_label(&run.status)
        );
    }
}

/// Prints one approval decision.
fn print_approval(response: &rpc::WorkflowImproveApproveResponse) {
    println!(
        "candidate {} {} by {}",
        response.candidate_id,
        if response.approved {
            "APPROVED"
        } else {
            "REJECTED"
        },
        response.approver
    );
    println!("  workflow:     {}", response.workflow);
    println!("  incumbent:    {}", response.incumbent_version_id);
    println!("  validation:   {}", response.validation_digest);
    if let Some(note) = response.note.as_deref() {
        println!("  note:         {note}");
    }
    if let Some(reason) = response.reason.as_deref() {
        println!("  reason:       {reason}");
    }
}

/// Parses one `--carry` value: `Name` or `Name <contact>`.
fn parse_attribution(value: &str) -> Result<rpc::WorkflowForkAttribution> {
    let trimmed = value.trim();
    let parsed = if let Some((name, contact)) = trimmed
        .strip_suffix('>')
        .and_then(|rest| rest.split_once('<'))
    {
        let name = name.trim();
        let contact = contact.trim();
        if name.is_empty() || contact.is_empty() {
            None
        } else {
            Some(rpc::WorkflowForkAttribution {
                name: name.to_string(),
                contact: Some(contact.to_string()),
            })
        }
    } else if !trimmed.is_empty() && !trimmed.contains(['<', '>']) {
        Some(rpc::WorkflowForkAttribution {
            name: trimmed.to_string(),
            contact: None,
        })
    } else {
        None
    };
    parsed.with_context(|| format!("attribution `{value}` must be `Name` or `Name <contact>`"))
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
        if let Some(text) = trimmed
            .strip_prefix("i ")
            .or_else(|| trimmed.strip_prefix("i:"))
        {
            instructions.push(text.trim().to_string());
        } else if let Some(text) = trimmed
            .strip_prefix("d ")
            .or_else(|| trimmed.strip_prefix("d:"))
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
                println!(
                    "  terminal: {}",
                    terminal_kind_label(&response.terminal.kind)
                );
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

/// The lowercase label of an improvement change kind.
fn change_kind_label(kind: &rpc::WorkflowChangeKind) -> &'static str {
    match kind {
        rpc::WorkflowChangeKind::DefinitionDelta => "definition-delta",
        rpc::WorkflowChangeKind::CapabilityBinding => "capability-binding",
        rpc::WorkflowChangeKind::RecoveryPolicy => "recovery-policy",
        rpc::WorkflowChangeKind::DependencyChoice => "dependency-choice",
        rpc::WorkflowChangeKind::ScheduleTuning => "schedule-tuning",
    }
}

/// The lowercase label of an improvement validation stage.
fn validation_stage_label(stage: &rpc::WorkflowValidationStageName) -> &'static str {
    match stage {
        rpc::WorkflowValidationStageName::Replay => "replay",
        rpc::WorkflowValidationStageName::Differential => "differential",
        rpc::WorkflowValidationStageName::Policy => "policy",
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
