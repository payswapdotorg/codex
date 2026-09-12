# Step 2 deviation — channel-name mismatch (catalog vs fixture)

The catalog's breaking-news fast path says "publish to the fast channels
(web + push; skip licensed partner-app)". The PressRoom fixture (media
seed @ 0d314fd09cec70d000f33c81286edcd8cbf72501) exposes five channels:
web-front-page, morning-newsletter, rss-feed, regional-syndication,
partner-app (the licensed one, expired 2026-06-01). There is NO "push"
channel. Observed instead (fast-path intent preserved): publish to
web-front-page + rss-feed, skip the licensed partner-app AND the slower
morning-newsletter/regional-syndication. Catalog/fixture naming mismatch
recorded (harness-level finding), not worked around silently.
