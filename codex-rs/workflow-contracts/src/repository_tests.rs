use super::*;
use crate::MANIFEST_FORMAT_VERSION;
use crate::RepositoryRelativePath;
use pretty_assertions::assert_eq;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER_COMMIT: &str = "89abcdef0123456789abcdef0123456789abcdef";

fn repository() -> WorkflowRepository {
    WorkflowRepository {
        identity: WorkflowRepositoryId::parse("https://github.com/acme/release-bot")
            .expect("valid repo"),
        forge: ForgeKind::parse(ForgeKind::GITHUB).expect("valid forge kind"),
        origin: Some(
            WorkflowRepositoryId::parse("https://github.com/acme/upstream-bot")
                .expect("valid repo"),
        ),
        forked_from: Some(crate::ForkLineage {
            upstream: WorkflowRepositoryId::parse("https://github.com/acme/upstream-bot")
                .expect("valid repo"),
            forked_at: Some(RevisionSha::parse(COMMIT).expect("valid sha")),
        }),
        default_branch: DevelopmentRef::branch("main").expect("valid branch"),
        branches: vec![WorkflowBranchState {
            branch: DevelopmentRef::branch("main").expect("valid branch"),
            head: Some(RevisionSha::parse(COMMIT).expect("valid sha")),
        }],
        workflows: vec![WorkflowManifest {
            manifest_version: MANIFEST_FORMAT_VERSION,
            workflow: WorkflowDefinitionId::parse("release-notes").expect("valid workflow id"),
            display_name: Some("Release Notes".to_owned()),
            description: None,
            repository: WorkflowRepositoryId::parse("https://github.com/acme/release-bot")
                .expect("valid repo"),
            definition_path: RepositoryRelativePath::parse(
                "workflows/release-notes/definition.json",
            )
            .expect("valid path"),
            provenance: None,
        }],
        maintainers: vec![crate::Attribution {
            name: "Ada Lovelace".to_owned(),
            contact: None,
        }],
    }
}

#[test]
fn repository_records_round_trip_through_serde() {
    let repository = repository();

    let serialized = serde_json::to_string(&repository).expect("repository serializes");
    let parsed: WorkflowRepository = serde_json::from_str(&serialized).expect("repository parses");

    assert_eq!(repository, parsed);
}

#[test]
fn repository_records_lookup_manifests_by_workflow() {
    let repository = repository();
    let workflow = WorkflowDefinitionId::parse("release-notes").expect("valid workflow id");

    let manifest = repository
        .manifest_for(&workflow)
        .expect("manifest for declared workflow");
    assert_eq!(manifest.workflow, workflow);

    let missing = WorkflowDefinitionId::parse("unknown").expect("valid workflow id");
    assert!(repository.manifest_for(&missing).is_none());
}

#[test]
fn reviews_pin_immutable_base_and_head() {
    let review = WorkflowReview {
        review: ReviewId::parse("review-7").expect("valid review id"),
        repository: WorkflowRepositoryId::parse("https://github.com/acme/release-bot")
            .expect("valid repo"),
        title: Some("Retry logic for flaky checks".to_owned()),
        base: ImmutableSourceRevision::pin_commit(RevisionSha::parse(COMMIT).expect("valid sha")),
        head: ImmutableSourceRevision::pin_commit(
            RevisionSha::parse(OTHER_COMMIT).expect("valid sha"),
        ),
        state: ReviewState::Open,
    };

    let serialized = serde_json::to_string(&review).expect("review serializes");
    let parsed: WorkflowReview = serde_json::from_str(&serialized).expect("review parses");

    assert_eq!(review, parsed);
    assert_eq!(parsed.base.commit_sha.as_str(), COMMIT);
    assert_eq!(parsed.head.commit_sha.as_str(), OTHER_COMMIT);
}

#[test]
fn merged_reviews_record_immutable_merge_revisions() {
    let review = WorkflowReview {
        state: ReviewState::Merged {
            merge_revision: RevisionSha::parse(OTHER_COMMIT).expect("valid sha"),
        },
        ..review_open()
    };

    let serialized = serde_json::to_value(&review).expect("review serializes");
    assert_eq!(
        serialized["state"],
        serde_json::json!({ "merged": { "mergeRevision": OTHER_COMMIT } })
    );
}

fn review_open() -> WorkflowReview {
    WorkflowReview {
        review: ReviewId::parse("review-7").expect("valid review id"),
        repository: WorkflowRepositoryId::parse("https://github.com/acme/release-bot")
            .expect("valid repo"),
        title: None,
        base: ImmutableSourceRevision::pin_commit(RevisionSha::parse(COMMIT).expect("valid sha")),
        head: ImmutableSourceRevision::pin_commit(
            RevisionSha::parse(OTHER_COMMIT).expect("valid sha"),
        ),
        state: ReviewState::Open,
    }
}

#[test]
fn releases_anchor_to_immutable_targets_not_tag_names() {
    let release = WorkflowRelease {
        tag: "v1.4.2".to_owned(),
        repository: WorkflowRepositoryId::parse("https://github.com/acme/release-bot")
            .expect("valid repo"),
        target: ImmutableSourceRevision::pin(
            RevisionSha::parse(COMMIT).expect("valid sha"),
            DevelopmentRef::tag("v1.4.2").ok(),
        ),
        versions: vec![WorkflowVersionId::from_digest(
            crate::ContentDigest::of(&serde_json::json!({ "workflow": "release-notes" }))
                .expect("digest"),
        )],
    };

    let serialized = serde_json::to_string(&release).expect("release serializes");
    let parsed: WorkflowRelease = serde_json::from_str(&serialized).expect("release parses");

    assert_eq!(release, parsed);
    // The tag name is provenance; the commit anchor is the authority.
    assert_eq!(parsed.target.commit_sha.as_str(), COMMIT);
    assert_eq!(
        parsed.target.resolved_from,
        DevelopmentRef::tag("v1.4.2").ok()
    );
}

#[test]
fn branch_states_are_development_snapshots_not_authority() {
    let repository = repository();

    // A moved branch would snapshot a different head; the recorded value is
    // just a snapshot. The branch state type offers no path into version
    // identity: it never appears in ExecutionVersionIdentity.
    let serialized = serde_json::to_string(&repository.branches).expect("branch states serialize");
    assert!(serialized.contains(COMMIT));
    let identity_fields = [
        "workflow",
        "semanticVersion",
        "repository",
        "sourceRevision",
        "definitionDigest",
        "dependencyLockDigest",
    ];
    for field in identity_fields {
        assert!(
            !serialized.contains(field),
            "branch snapshots must not overlap version identity field `{field}`"
        );
    }
}

/// A minimal in-memory forge implementation used to prove the trait is
/// implementable without forge-specific semantics.
struct MemoryForge {
    kind: ForgeKind,
    head: RevisionSha,
}

impl WorkflowForge for MemoryForge {
    fn kind(&self) -> &ForgeKind {
        &self.kind
    }

    fn resolve_ref(
        &self,
        repository: &WorkflowRepositoryId,
        reference: &DevelopmentRef,
    ) -> impl Future<Output = Result<ImmutableSourceRevision, WorkflowContractError>> + Send {
        let revision = ImmutableSourceRevision::pin(self.head.clone(), Some(reference.clone()));
        let _ = repository;
        std::future::ready(Ok(revision))
    }

    fn repository_record(
        &self,
        repository: &WorkflowRepositoryId,
    ) -> impl Future<Output = Result<WorkflowRepository, WorkflowContractError>> + Send {
        let record = WorkflowRepository {
            identity: repository.clone(),
            forge: self.kind.clone(),
            origin: None,
            forked_from: None,
            default_branch: DevelopmentRef::branch("main").expect("valid branch"),
            branches: vec![],
            workflows: vec![],
            maintainers: vec![],
        };
        std::future::ready(Ok(record))
    }
}

#[tokio::test]
async fn forge_trait_is_implementable_without_forge_semantics() {
    let forge = MemoryForge {
        kind: ForgeKind::parse("memory").expect("valid kind"),
        head: RevisionSha::parse(COMMIT).expect("valid sha"),
    };
    let repository = WorkflowRepositoryId::parse("https://git.example.com/acme/release-bot")
        .expect("valid repo");

    let resolved = forge
        .resolve_ref(
            &repository,
            &DevelopmentRef::branch("main").expect("valid branch"),
        )
        .await
        .expect("ref resolves");
    assert_eq!(resolved.commit_sha.as_str(), COMMIT);
    assert_eq!(
        resolved.resolved_from,
        DevelopmentRef::branch("main").ok(),
        "resolution records the development ref as provenance"
    );

    let record = forge
        .repository_record(&repository)
        .await
        .expect("record reads");
    assert_eq!(record.forge.as_ref(), "memory");
    assert_eq!(record.default_branch.name(), "main");
}
