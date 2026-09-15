#!/usr/bin/env bash
# seed-workspace.sh — deterministic NST-D workspace materializer (VWO-012).
# Purpose-built proving-ground setup per BENCHMARK-CATALOG §5 note 4 +
# VALIDATION-PROGRAM §4: seeds the scratch trees the NST-D tasks bind to.
# Fixture clock: state.meta.today = 2026-09-12 (catalog §5 note 6).
set -euo pipefail

WS="${1:-/tmp/nstd-workspace}"
rm -rf "$WS"
mkdir -p "$WS"/{field-notes,team-notes,site-photos,weekly,.autosave}

# --- NST-D006: seeded field-notes scratch tree (last week's notes + scratch) ---
cat > "$WS/field-notes/note-2026-09-08.txt" << 'EOF'
Morning: crane B pre-check done, all clear.
Afternoon: slab pour started on level 12. Weather held.
EOF
cat > "$WS/field-notes/note-2026-09-09.txt" << 'EOF'
Level 12 slab cured enough for foot traffic.
Rebar delivery for level 13 arrived late (14:40) — unloaded by 16:10.
EOF
cat > "$WS/field-notes/note-2026-09-10.txt" << 'EOF'
Formwork crew started level 13 east side.
Safety walk: one guardrail strap missing on the east edge — fixed same hour.
EOF
cat > "$WS/field-notes/note-2026-09-11.txt" << 'EOF'
Formwork 60% complete on level 13.
Inspector visit booked for Monday.
EOF
cat > "$WS/field-notes/note-2026-09-12.txt" << 'EOF'
Formwork complete on level 13 east + west.
Crane B certified and released for ops.
EOF
cat > "$WS/field-notes/scratch-todo.txt" << 'EOF'
TODO call concrete supplier
TODO confirm inspector time Monday
TODO order guardrail straps (spare)
EOF
mkdir -p "$WS/field-notes/old-archive"
cat > "$WS/field-notes/old-archive/note-2026-08-28.txt" << 'EOF'
Retained: level 10 closeout sign-off notes.
EOF

# --- NST-D009: seeded site photos (deterministic via ffmpeg test sources) ---
# 3 landscape 1920x1080 + 1 sideways 1080x1920 (the one to rotate)
PHOTO_SIZES=("1920x1080" "1920x1080" "1920x1080" "1080x1920")
PHOTO_NAMES=("site-east-formwork" "site-crane-b" "site-level13-slab" "site-safety-walk-sideways")
for i in 0 1 2 3; do
  ffmpeg -y -hide_banner -loglevel error -f lavfi -i "testsrc2=size=${PHOTO_SIZES[$i]}:rate=1:duration=1" \
    -vf "drawtext=text='${PHOTO_NAMES[$i]}':x=(w-text_w)/2:y=(h-text_h)/2:fontsize=48:fontcolor=white" \
    -frames:v 1 "$WS/site-photos/${PHOTO_NAMES[$i]}.png"
done

# --- NST-D005: weekly folder (export target) ---
cat > "$WS/weekly/README.txt" << 'EOF'
Weekly report folder — drop the exported project report here every Friday.
EOF

# --- NST-D007: team-notes landing dir ---
cat > "$WS/team-notes/README.txt" << 'EOF'
Team notes: safety briefings and day docs are saved here by the foreman.
EOF

echo "workspace seeded at $WS"
find "$WS" -type f | sort
