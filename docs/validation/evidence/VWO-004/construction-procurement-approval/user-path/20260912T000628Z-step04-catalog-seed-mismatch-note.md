# Step 4 — "decide the other open request (valid vendor)" NOT PERFORMABLE as written (catalog/seed mismatch)

Catalog user-path step 4: "Decide the other open request (the one with a
valid vendor) — approve; observe the PO issued and the notification."

Current VWO-002 seed (construction/seed.js @ 0d314fd09cec70d000f33c81286edcd8cbf72501)
contains exactly two purchase requests:
- prq-5001 — status `submitted`, vendor SteelCo Fabrication, contract
  EXPIRED 2026-08-01 (natural stale_entitlement; approve is hard-blocked);
- prq-5002 — vendor contract VALID (until 2027-01-15) but status
  `po_issued` (already decided; PO po-2041 exists; no decide form rendered).

There is NO second OPEN request with a valid vendor, and no UI or API to
create a new purchase request in this fixture. Therefore the approve->PO
success path CANNOT be exercised by a normal person through the current
product surface in this scenario. Observed instead: the seeded PO po-2041
is visible on /procurement (PO table) — a static, not live, completion.

Recorded as a harness/catalog finding (catalog references a seed state that
does not exist; see report findings). Not simulated: no PO was fabricated.
