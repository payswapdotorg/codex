# Pack Client-Contract Manifest

**Status:** AUTHORITATIVE WIRE REFERENCE (PACK Wave 2, Worker C lane)
**Source of truth:** `codex-rs/pack-contracts/src/` (the Rust types) and
`codex-rs/pack-contracts/tests/fixtures/` (golden wire exemplars, byte-stable).

This manifest is the field-by-field wire contract every client of the Pack
contracts must consume — today that is the Flauz.app desktop client
(PACK-UX-001); future clients include web, mobile, browser-extension, IDE,
and SDK surfaces (per the frozen client-adapter architecture). It is
derived mechanically from the real types; the golden fixtures enforce it.

## Consumption rules (non-negotiable)

1. **Clients consume; they do not own.** A client renders pack state and
   requests changes through explicit contracts. It never recomputes
   digests, never verifies platform integrity (display digests as opaque
   strings), and never becomes durable pack authority.
2. **Wire format:** canonical JSON, `camelCase` field names,
   `deny_unknown_fields` on every record — an unknown key is a wire
   violation, not a tolerated extra.
3. **Optional fields are `skip_serializing_if`**: absent means absent.
   Legacy (pre-PACK-005) artifacts carry NO `composition*` keys at all.
4. **Digests and identities** are opaque `sha256:<hex>` strings to
   clients. Never parse meaning out of them; display them.
5. **Candidate vs promoted are distinct record kinds.** The same governed
   content yields different `revisionId`s per kind. A UI must never
   conflate them (see `recordKind` note below).

## Record shapes

### Candidate pack record (`candidate-root.json`, `candidate-child.json`, `composed-candidate.json`)

```text
packId                 string   pack lineage id (validated slug)
semanticVersion        string   "major.minor.patch"
parentRevision?        object   { revision: <id>, label?: string }   (absent on roots)
mission                object   mission model (below)
missionDigest          string   sha256 of mission content
policySet              object   constitution + governing policies
policyDigest           string   sha256 of policy content
dependencyLock         object   resolved dependency map (below)
dependencyLockDigest   string   sha256 of lock content
systemState            object   reference sets (below)
systemStateDigest      string   sha256 of state content
provenance             object   { parentRevision?: <id>, producer: {...} }
composition?           object   two-parent composition record (below) — composed candidates only
compositionDigest?     string   sha256 of the composition record — composed candidates only
revisionId             string   content-addressed candidate identity
```

### Promoted pack record (`promoted.json`)

Identical field set, plus `promotedFrom: <candidate revisionId>`, and its
`revisionId` is the promoted identity (always distinct from the source
candidate's). There is no wire field for "record kind": the client knows
which record type it requested through the explicit contract method.

### Mission (`mission.json`)

```text
id                string
statement         string
author            { kind: "user" | "agentProposal", subject: string }
valueModel        { objectives: [ { statement } ] }
contextModel      { notes: [ { statement } ] }              // notes power contextual activation
hardConstraints   [ { statement } ]
preferences       [ { statement } ]
successMeasures   [ { statement } ]
```

`author.kind` distinguishes user authority from agent proposals — a client
MUST render this distinction; an agent-proposed mission is never silent
mission authority.

### Policy set (`policy-set.json`)

```text
constitution  { rules: [ rule... ], protectedInvariants: [ invariant... ] }
policies      [ { scope, statements: [string], authorityBoundary: {...}, referencedPolicies? } ]
```

Rule variants (tagged): `forbiddenAction`, `requiredApproval`,
`provenanceRequirement`, `dataIntegrity`, `domain`, `auditRequirement`,
each `{ statement }`. `protectedInvariants` is always the full platform
set — a constitution can never weaken platform invariants.

### Dependency lock (`dependency-lock.json`)

```text
entries: {
  "<workflow|capability|pack>:<snake_case_dependency_id>": {
    key           string     repeats the map key
    resolved      object     { workflowVersion: <id> } | { capability: <id> } | { packRevision: <id> }
    contentDigest string
    provenance?   object
  }
}
```

A dependency key maps to exactly ONE immutable identity; re-pinning is a
new revision, never a mutation.

### System state (`system-state.json`)

```text
workflowVersionRefs  [ { workflowVersionId: <id>, role?: string } ]
capabilityRefs       [ { capability: <id>, constraint?: string } ]
policyRefs           [ { policyId: <id>, validatedContentDigest: string } ]
evaluationRefs       [ string ]   // opaque content addresses
evidenceRefs         [ string ]   // opaque content addresses
rollbackCheckpoint?  { targetRevision: <id>, stateDigest: string, reason: string }
```

All vectors are canonical (sorted by identity, duplicate-free). Roles and
constraints are descriptive text.

### Assurance policy (`assurance-policy.json`)

```text
determinism?        { level }
replay?             { scope }
approval?           { approverClass, threshold }
evidence?           { level }
modelPinning?       { pin: { model, digest } }
dependencyPinning?  { pin: { dependency, digest } }
environmentPinning? { pin: { environment, digest } }
```

Seven INDEPENDENT dimensions; an absent dimension declares no requirement
(there is no global deterministic mode). Pins carry identity + content
digest — never credentials.

### Composition record (inside composed candidates)

```text
base         full revision identity tuple of the base parent
overlay      full revision identity tuple of the overlay parent
relation     "specialization" | "orthogonal"
strategy     { activationMerge: "requireCompatible" | "union" }
activations? [ { target: { workflowVersion: <id> } | { capability: <id> },
                 scope: "always" | { whenContextNote: <statement> } } ]
```

Absent `composition` = a non-composed revision (every pre-PACK-005
artifact). Two parents, one new candidate — parents are never mutated.

### Revision identity tuple (inside `composition.base`/`overlay`)

```text
pack                   string
semanticVersion        string
systemStateDigest      string
missionDigest          string
policyDigest           string
dependencyLockDigest   string
parentRevision?        string
compositionDigest?     string    // only when the parent was itself composed
```

## Client guidance (PACK-UX-001)

- Render `candidate` and `promoted` as visually distinct lifecycle states;
  the record shape tells you which one you hold.
- Render `mission.author.kind` (user authority vs agent proposal).
- Render `composition` provenance as lineage (two parents + relation +
  strategy); absent composition renders as normal single-parent lineage.
- Render assurance dimensions independently; absence means "no
  requirement", NOT "non-deterministic mode".
- Digests are opaque strings: display, never verify, never derive.
- Unknown fields in a response are a contract violation: surface it, do
  not silently ignore it.

## Compatibility policy

Any wire change to these shapes lands with: (a) a golden-fixture
regeneration, (b) a manifest update, and (c) a note for consuming clients.
The fixtures are the enforcement: `cargo test -p codex-pack-contracts
--test golden_fixtures` fails on any drift.
