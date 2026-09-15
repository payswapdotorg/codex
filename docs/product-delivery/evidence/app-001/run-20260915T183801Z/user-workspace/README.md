# Scratch user workspace (git-trimmed)

The teach pipeline ran from this scratch git repository (git init + one
empty initial commit). `codex workflow teach --yes` pinned its publish to
the repository HEAD — commitSha recorded in `../teach.json` (publish
stage). The scratch repo's objects were stripped before commit (embedded
git repos cannot be carried in evidence trees); its exact state is
reproducible from the transcript and the pinned commitSha.
