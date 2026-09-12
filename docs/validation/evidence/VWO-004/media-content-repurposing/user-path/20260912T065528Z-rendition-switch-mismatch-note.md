# Catalog/fixture switch mismatch — "missing_asset on rendition" not implemented

SCENARIO-CATALOG.md (media-content-repurposing, "Failure switches & recovery
expectation") lists: "`missing_asset` on rendition (referenced asset
unresolvable) — rendition fails with asset id, recovery = fix reference,
re-request".

The PressRoom fixture at 0d314fd09cec70d000f33c81286edcd8cbf72501 implements
NO missing_asset branch in the rendition operation
(docs/validation/fixtures/media/ops-dist.js, requestRendition): the declared
rendition failure switch is `data_conflict` (in-flight job conflict, HTTP 409
naming the asset id), and the fixture's own switch table
(docs/validation/fixtures/media/server.js + FAILURES.md) binds missing_asset
to "attach asset, publish story" only.

Observed (user-path evidence, this directory):
- `…switch-missing-asset-not-applied-to-rend…txt` — with
  X-Failure-Switch: missing_asset armed, the rendition request for a-0302
  SUCCEEDED ("Rendition \"social-card\" rendered for a-0302.") — the switch
  does not apply to this operation.
- `…switch-data-conflict-rendition-flash.txt` / `…recovery.txt` — the
  implemented rendition switch (data_conflict) exercised instead: 409
  "Rendition request conflicts with an in-flight job for a-0301 (simulated).
  [data_conflict]" naming the asset id, then clean retry → rendition ready
  (the catalog's observable intent: named-asset failure + re-request
  recovery).
- post-hoc pull — natural unresolvable-asset rendition request (API twin,
  nonexistent asset id) returns 404 asset_not_found naming the id: the
  "referenced asset unresolvable" case exists naturally, but is not
  reachable from the /assets UI (the combobox only offers existing assets).

Recorded as a harness-level catalog/fixture consistency finding (see report);
not worked around silently and not simulated.
