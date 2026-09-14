#!/usr/bin/env python3
"""RWO-009 predicate-logic model check (python transcription, NOT compiled Rust).

Transcribes, line-faithfully, the control flow of
codex-rs/workflow-distribution/src/{discovery,memory}.rs at BASE
(2d37cc0ef) and on the rwo-009 branch, and drives the VWO-009 engine
probe attack scenarios (6d / 12) through both. The BEFORE and AFTER
runs use IDENTICAL scenario state (same releases, same installs, same
seeded access grants) so the contrast is purely the code change.

Purpose: mechanically verify the DECISION LOGIC of the guards (branch
outcomes, gate order, side-effect points) while `cargo` is absent from
this sandbox. This is NOT a substitute for
`cargo test -p codex-workflow-distribution` — the Tech Lead compiles
and runs the real crate (the new unit tests in src/memory_tests.rs
encode these same scenarios as Rust assertions, with the access grant
deliberately ABSENT there so the asserted error type proves which
guard fired; here the grant is PRESENT so BASE reproduces the probe's
applied=true hole exactly).

run: python3 rwo009-model-check.py
"""
from dataclasses import dataclass, field


# --- discovery.rs :: release_installable (identical at BASE and on the branch) ---
def release_installable(metadata, state, audience, requesting):
    return (
        metadata["ownership"] == requesting
        or state == "Public"                      # is_publicly_discoverable
        or state == "Published"
        or (state == "Shared" and requesting in audience)
    )


# --- the in-memory marketplace model -------------------------------------
@dataclass
class Entry:
    metadata: dict
    state: str
    audience: list


@dataclass
class Install:
    principal: str
    installed: dict          # PublishedVersionRef: {"version_id", "semver"}
    upgrade_policy: str      # "pin" | "follow"


@dataclass
class Market:
    entries: dict = field(default_factory=dict)   # version_id -> Entry
    installs: dict = field(default_factory=dict)  # workflow -> Install
    access_allowed: set = field(default_factory=set)  # InMemoryAccessPolicy seeds
    history: list = field(default_factory=list)   # UpgradeRecords


def evaluate_upgrade_base(market, workflow):
    install = market.installs[workflow]
    if install.upgrade_policy == "pin":
        return None
    newest = None
    for vid, entry in market.entries.items():
        if (
            entry.metadata["workflow"] == workflow
            and vid != install.installed["version_id"]
        ):
            if newest is None or entry.metadata["semver"] > market.entries[newest].metadata["semver"]:
                newest = vid
    return market.entries[newest].metadata["release"] if newest else None


def evaluate_upgrade_fixed(market, workflow):
    install = market.installs[workflow]
    if install.upgrade_policy == "pin":
        return None
    newest = None
    for vid, entry in market.entries.items():
        if (
            entry.metadata["workflow"] == workflow
            and vid != install.installed["version_id"]
            and release_installable(entry.metadata, entry.state, entry.audience, install.principal)
        ):
            if newest is None or entry.metadata["semver"] > market.entries[newest].metadata["semver"]:
                newest = vid
    return market.entries[newest].metadata["release"] if newest else None


def decide_upgrade_base(market, proposal, decision):
    # stale check -> reject path -> approval: integrity -> gates -> apply
    # BASE: NO ordering guard, NO visibility gate on the approval path.
    workflow = proposal["workflow"]
    install = market.installs[workflow]
    if install.installed["version_id"] != proposal["from"]:
        return ("Err", "StaleUpgradeProposal")
    if decision == "Reject":
        market.history.append({"from": proposal["from"], "to": proposal["from"], "applied": False})
        return ("Ok", {"applied": False, "to": proposal["from"]})
    if proposal["to"]["version_id"] not in market.access_allowed:
        return ("Err", "UpgradeRefused")
    market.installs[workflow] = Install(install.principal, proposal["to"], install.upgrade_policy)
    market.history.append({"from": proposal["from"], "to": proposal["to"]["version_id"], "applied": True})
    return ("Ok", {"applied": True, "to": proposal["to"]["version_id"]})


def decide_upgrade_fixed(market, proposal, decision):
    # stale check -> reject path -> APPROVAL PATH (branch order):
    #   (1) ordering guard  (2) visibility gate  (3) integrity
    #   (4) access/entitlement gates -> apply
    workflow = proposal["workflow"]
    install = market.installs[workflow]
    if install.installed["version_id"] != proposal["from"]:
        return ("Err", "StaleUpgradeProposal")
    if decision == "Reject":
        market.history.append({"from": proposal["from"], "to": proposal["from"], "applied": False})
        return ("Ok", {"applied": False, "to": proposal["from"]})
    if proposal["to"]["semver"] < install.installed["semver"]:
        return ("Err", "IllegalDowngrade", install.installed["semver"], proposal["to"]["semver"])
    entry = market.entries[proposal["to"]["version_id"]]
    if not release_installable(entry.metadata, entry.state, entry.audience, install.principal):
        return ("Err", "ReleaseNotVisible", proposal["to"]["version_id"])
    if proposal["to"]["version_id"] not in market.access_allowed:
        return ("Err", "UpgradeRefused")
    market.installs[workflow] = Install(install.principal, proposal["to"], install.upgrade_policy)
    market.history.append({"from": proposal["from"], "to": proposal["to"]["version_id"], "applied": True})
    return ("Ok", {"applied": True, "to": proposal["to"]["version_id"]})


# --- scenario construction (IDENTICAL state for BASE and FIXED runs) -------
def scenario_6d():
    # VWO-009 engine probe attack 6d: foreign installer (bob) pins a public
    # 1.0.0; the owner (acme-ops) publishes a NEWER PRIVATE 2.0.0. bob holds
    # host-seeded license acceptances for both releases (the access gate is
    # NOT the hole — the missing visibility gate is).
    m = Market()
    m.entries["sha256:v1-public-1.0.0"] = Entry(
        {"workflow": "wo", "semver": (1, 0, 0), "ownership": "acme-ops",
         "release": {"version_id": "sha256:v1-public-1.0.0", "semver": (1, 0, 0)}},
        "Public", [])
    m.entries["sha256:v2-private-2.0.0"] = Entry(
        {"workflow": "wo", "semver": (2, 0, 0), "ownership": "acme-ops",
         "release": {"version_id": "sha256:v2-private-2.0.0", "semver": (2, 0, 0)}},
        "Private", [])
    m.installs["wo"] = Install("bob", {"version_id": "sha256:v1-public-1.0.0", "semver": (1, 0, 0)}, "follow")
    m.access_allowed = {"sha256:v1-public-1.0.0", "sha256:v2-private-2.0.0"}
    return m


def scenario_12():
    # VWO-009 engine probe attack 12: alice pins public 2.0.0; an OLDER
    # public 1.5.0 exists; follow policy; alice holds license acceptances
    # for both (at base the apply path ran to completion: pin 2.0.0 -> 1.5.0).
    m = Market()
    m.entries["sha256:v2-public-2.0.0"] = Entry(
        {"workflow": "wo", "semver": (2, 0, 0), "ownership": "acme-ops",
         "release": {"version_id": "sha256:v2-public-2.0.0", "semver": (2, 0, 0)}},
        "Public", [])
    m.entries["sha256:v1-public-1.5.0"] = Entry(
        {"workflow": "wo", "semver": (1, 5, 0), "ownership": "acme-ops",
         "release": {"version_id": "sha256:v1-public-1.5.0", "semver": (1, 5, 0)}},
        "Public", [])
    m.installs["wo"] = Install("alice", {"version_id": "sha256:v2-public-2.0.0", "semver": (2, 0, 0)}, "follow")
    m.access_allowed = {"sha256:v2-public-2.0.0", "sha256:v1-public-1.5.0"}
    return m


def fmt_semver(v):
    return "%d.%d.%d" % v


def fmt_result(r):
    if r[0] == "Err":
        if r[1] == "IllegalDowngrade":
            return "Err(IllegalDowngrade { expected_newer_than: %s, got: %s })" % (
                fmt_semver(r[2]), fmt_semver(r[3]))
        if r[1] == "ReleaseNotVisible":
            return "Err(ReleaseNotVisible { version: %s })" % r[2]
        return "Err(%s)" % r[1]
    return "Ok(UpgradeRecord { applied: %s, to: %s })" % (r[1]["applied"], r[1]["to"])


overall = []

print("RWO-009 predicate-logic model check (python transcription of the Rust control flow)")
print("base: 2d37cc0ef memory.rs/discovery.rs  |  fixed: rwo-009/upgrade-visibility-gate branch")
print("before/after runs share IDENTICAL scenario state (releases, installs, access seeds)")
print("NOTE: model of decision logic only — the real crate must still be compiled+tested")
print("      (cargo absent in this sandbox; the Tech Lead runs cargo test/clippy independently)")
print()

for name, scenario, attack, base_expect, fixed_expect in [
    ("ATTACK 6d (Family I, VWO-009 F10)", scenario_6d,
     {"workflow": "wo", "from": "sha256:v1-public-1.0.0",
      "to": {"version_id": "sha256:v2-private-2.0.0", "semver": (2, 0, 0)}},
     "hole: evaluate leaks private identity; decide applies it",
     "refused: evaluate None; decide ReleaseNotVisible; pin unchanged"),
    ("ATTACK 12 (Family G engine half, VWO-009 F4)", scenario_12,
     {"workflow": "wo", "from": "sha256:v2-public-2.0.0",
      "to": {"version_id": "sha256:v1-public-1.5.0", "semver": (1, 5, 0)}},
     "hole: pin moves 2.0.0 -> 1.5.0 via applied record",
     "refused: decide IllegalDowngrade; pin unchanged; no record"),
]:
    print("=" * 78)
    print(name)
    print("=" * 78)
    m_base, m_fixed = scenario(), scenario()

    print("-- evaluate_upgrade(workflow) --")
    p_base = evaluate_upgrade_base(m_base, "wo")
    p_fixed = evaluate_upgrade_fixed(m_fixed, "wo")
    print("   BASE : %s" % (
        "None" if p_base is None else "Some(proposal.to = %s, semver %s)"
        % (p_base["version_id"], fmt_semver(p_base["semver"]))))
    print("   FIXED: %s" % (
        "None" if p_fixed is None else "Some(proposal.to = %s, semver %s)"
        % (p_fixed["version_id"], fmt_semver(p_fixed["semver"]))))
    leak_base = p_base is not None and "private" in p_base["version_id"]
    leak_fixed = p_fixed is not None and "private" in p_fixed["version_id"]
    print("   private identity surfaced to foreign installer? BASE=%s FIXED=%s"
          % (leak_base, leak_fixed))

    print("-- decide_upgrade(same proposal, Approve) --")
    r_base = decide_upgrade_base(m_base, attack, "Approve")
    r_fixed = decide_upgrade_fixed(m_fixed, attack, "Approve")
    print("   BASE : %s" % fmt_result(r_base))
    print("   FIXED: %s" % fmt_result(r_fixed))

    print("-- post-state --")
    for label, m in (("BASE", m_base), ("FIXED", m_fixed)):
        pin = m.installs["wo"].installed["version_id"]
        print("   %s: pin=%s  upgrade_history=%d record(s)  applied=%d"
              % (label, pin, len(m.history),
                 sum(1 for h in m.history if h["applied"])))
    pin_base = m_base.installs["wo"].installed["version_id"]
    pin_fixed = m_fixed.installs["wo"].installed["version_id"]

    if name.startswith("ATTACK 6d"):
        ok = (leak_base and not leak_fixed
              and r_fixed[0] == "Err" and r_fixed[1] == "ReleaseNotVisible"
              and pin_fixed == attack["from"] and len(m_fixed.history) == 0)
    else:
        ok = (r_base[0] == "Ok" and r_base[1]["applied"]
              and pin_base == attack["to"]["version_id"]
              and r_fixed[0] == "Err" and r_fixed[1] == "IllegalDowngrade"
              and pin_fixed == attack["from"] and len(m_fixed.history) == 0)
    overall.append((name, ok))
    print("   VERDICT: BASE reproduced the VWO-009 hole; FIXED refused, pin unchanged,")
    print("             nothing recorded -> %s" % ("PASS" if ok else "MODEL FAILURE"))
    print()

print("=" * 78)
print("existing-behavior parity checks on the FIXED model (e2e suite shapes):")
print("=" * 78)
# stale proposal (e2e line 1165): from != installed -> StaleUpgradeProposal, nothing recorded
m = scenario_6d()
r = decide_upgrade_fixed(m, {"workflow": "wo", "from": "sha256:wrong",
                             "to": {"version_id": "sha256:v2-private-2.0.0", "semver": (2, 0, 0)}}, "Approve")
ok1 = r == ("Err", "StaleUpgradeProposal") and len(m.history) == 0
print("   stale proposal        -> %s, history=%d  %s" % (r, len(m.history), "PASS" if ok1 else "FAIL"))
# reject path (e2e lines 1122-1136): records applied=false, to=from, pin stays
m = scenario_12()
r = decide_upgrade_fixed(m, {"workflow": "wo", "from": "sha256:v2-public-2.0.0",
                             "to": {"version_id": "sha256:v1-public-1.5.0", "semver": (1, 5, 0)}}, "Reject")
ok2 = (r[0] == "Ok" and not r[1]["applied"] and r[1]["to"] == "sha256:v2-public-2.0.0"
       and len(m.history) == 1 and not m.history[0]["applied"])
print("   reject decision       -> applied=False to=from, history=1  %s" % ("PASS" if ok2 else "FAIL"))
# legitimate visible forward upgrade (e2e lines 1138-1162): gates pass -> applied
m = Market()
m.entries["sha256:pub-1.0.0"] = Entry({"workflow": "wo", "semver": (1, 0, 0), "ownership": "acme-ops",
                                       "release": {"version_id": "sha256:pub-1.0.0", "semver": (1, 0, 0)}}, "Public", [])
m.entries["sha256:pub-1.1.0"] = Entry({"workflow": "wo", "semver": (1, 1, 0), "ownership": "acme-ops",
                                       "release": {"version_id": "sha256:pub-1.1.0", "semver": (1, 1, 0)}}, "Public", [])
m.installs["wo"] = Install("alice", {"version_id": "sha256:pub-1.0.0", "semver": (1, 0, 0)}, "follow")
m.access_allowed = {"sha256:pub-1.1.0"}
r = decide_upgrade_fixed(m, {"workflow": "wo", "from": "sha256:pub-1.0.0",
                             "to": {"version_id": "sha256:pub-1.1.0", "semver": (1, 1, 0)}}, "Approve")
ok3 = r[0] == "Ok" and r[1]["applied"] and r[1]["to"] == "sha256:pub-1.1.0"
print("   visible forward 1.0.0 -> 1.1.0 (public) -> applied=True  %s" % ("PASS" if ok3 else "FAIL"))
# pin policy never surfaces (e2e lines 1046-1052)
m = scenario_6d()
m.installs["wo"].upgrade_policy = "pin"
p = evaluate_upgrade_fixed(m, "wo")
ok4 = p is None
print("   pin policy evaluate   -> None  %s" % ("PASS" if ok4 else "FAIL"))
# entitlement-refused upgrade (e2e lines 945-1001): access ok, entitlement
# denies -> UpgradeRefused (the visibility gate must not shadow it)
m = Market()
m.entries["sha256:pub-1.0.0"] = Entry({"workflow": "wo", "semver": (1, 0, 0), "ownership": "acme-ops",
                                       "release": {"version_id": "sha256:pub-1.0.0", "semver": (1, 0, 0)}}, "Public", [])
m.entries["sha256:pub-2.0.0"] = Entry({"workflow": "wo", "semver": (2, 0, 0), "ownership": "acme-ops",
                                       "release": {"version_id": "sha256:pub-2.0.0", "semver": (2, 0, 0)}}, "Public", [])
m.installs["wo"] = Install("bob", {"version_id": "sha256:pub-1.0.0", "semver": (1, 0, 0)}, "follow")
m.access_allowed = set()  # license NOT accepted -> access gate refuses
r = decide_upgrade_fixed(m, {"workflow": "wo", "from": "sha256:pub-1.0.0",
                             "to": {"version_id": "sha256:pub-2.0.0", "semver": (2, 0, 0)}}, "Approve")
ok5 = r == ("Err", "UpgradeRefused") and len(m.history) == 0
print("   gate-refused upgrade  -> Err(UpgradeRefused), no record  %s" % ("PASS" if ok5 else "FAIL"))
print()
print("=" * 78)
failed = [n for n, ok in overall if not ok] + (
    [] if all([ok1, ok2, ok3, ok4, ok5]) else ["parity checks"])
print("SUMMARY: attack scenarios %s; parity checks %s"
      % ("PASS" if all(ok for _, ok in overall) else "FAIL " + str(failed),
         "PASS" if all([ok1, ok2, ok3, ok4, ok5]) else "FAIL"))
print("model check complete")
