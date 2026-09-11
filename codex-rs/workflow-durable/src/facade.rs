//! The builder/facade over one durable control-plane root directory.

use std::fs;
use std::path::Path;

use crate::DurableEvidenceStore;
use crate::DurableInstallationStore;
use crate::DurableInstanceControl;
use crate::DurableInstanceStore;
use crate::DurableRunPositionStore;
use crate::DurableStoreError;
use crate::DurableTriggerLedger;
use crate::DurableVersionStore;

/// The durable control-plane stores of one root directory.
///
/// Every store is an independent shared-state handle, so hosts can adopt
/// them piecemeal — the version store for the lifecycle, the ledger for
/// the trigger plane — while [`DurableStores::open`] wires all seven over
/// the canonical layout below. The instance store handle is shared with
/// [`DurableInstanceControl`], mirroring how the in-memory doubles
/// compose: the lifecycle, the trigger plane, and the resume seam
/// observe and mutate the same durable instance records.
///
/// ```text
/// <root>/versions.json            atomic snapshot (id -> sealed record)
/// <root>/instances.json           atomic snapshot (id -> instance record)
/// <root>/installations.json       atomic snapshot (workflow -> config)
/// <root>/run-positions.json      atomic snapshot (id -> run position)
/// <root>/evidence.jsonl           append-only journal (payload records)
/// <root>/triggers.jsonl           append-only journal (accept/settle)
/// <root>/install-audits.jsonl     append-only journal (rebind audits)
/// <root>/resume-directives.jsonl  append-only journal (resume audit)
/// ```
///
/// Re-opening the same root reconstructs the full control-plane state:
/// snapshots load atomically, journals replay in order, and torn tails
/// are repaired per the documented journal policy.
#[derive(Clone, Debug)]
pub struct DurableStores {
    /// Durable workflow version records.
    pub versions: DurableVersionStore,
    /// Durable workflow instance records (shared with `control`).
    pub instances: DurableInstanceStore,
    /// The durable evidence plane.
    pub evidence: DurableEvidenceStore,
    /// The durable trigger idempotency and audit ledger.
    pub ledger: DurableTriggerLedger,
    /// Durable installed configurations and rebind audits.
    pub installations: DurableInstallationStore,
    /// The durable resume seam over `instances`.
    pub control: DurableInstanceControl,
    /// Durable persisted run positions of active runs (MWO-003): what
    /// a resumed instance's continuation rehydrates from.
    pub run_positions: DurableRunPositionStore,
}

impl DurableStores {
    /// The canonical version snapshot file name.
    pub const VERSIONS_FILE: &'static str = "versions.json";
    /// The canonical instance snapshot file name.
    pub const INSTANCES_FILE: &'static str = "instances.json";
    /// The canonical installation snapshot file name.
    pub const INSTALLATIONS_FILE: &'static str = "installations.json";
    /// The canonical run-position snapshot file name.
    pub const RUN_POSITIONS_FILE: &'static str = "run-positions.json";
    /// The canonical evidence journal file name.
    pub const EVIDENCE_FILE: &'static str = "evidence.jsonl";
    /// The canonical trigger ledger journal file name.
    pub const TRIGGERS_FILE: &'static str = "triggers.jsonl";
    /// The canonical rebind audit journal file name.
    pub const INSTALL_AUDITS_FILE: &'static str = "install-audits.jsonl";
    /// The canonical resume directive journal file name.
    pub const RESUME_DIRECTIVES_FILE: &'static str = "resume-directives.jsonl";

    /// Opens (creating when absent) every durable store under `root`.
    ///
    /// All state found under the root is loaded before returning: a
    /// structurally corrupt committed record is a deterministic error
    /// from this constructor, never a silently empty store.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, DurableStoreError> {
        let root = root.as_ref();
        fs::create_dir_all(root)
            .map_err(|source| DurableStoreError::io("creating the durable store root", source))?;
        let instances = DurableInstanceStore::open(root.join(Self::INSTANCES_FILE))?;
        let control = DurableInstanceControl::open(
            instances.clone(),
            root.join(Self::RESUME_DIRECTIVES_FILE),
        )?;
        Ok(Self {
            versions: DurableVersionStore::open(root.join(Self::VERSIONS_FILE))?,
            instances,
            evidence: DurableEvidenceStore::open(root.join(Self::EVIDENCE_FILE))?,
            ledger: DurableTriggerLedger::open(root.join(Self::TRIGGERS_FILE))?,
            installations: DurableInstallationStore::open(
                root.join(Self::INSTALLATIONS_FILE),
                root.join(Self::INSTALL_AUDITS_FILE),
            )?,
            control,
            run_positions: DurableRunPositionStore::open(root.join(Self::RUN_POSITIONS_FILE))?,
        })
    }
}

#[cfg(test)]
#[path = "facade_tests.rs"]
mod tests;
