# Completed-slice records

Code comments cite completed slices as `§4.5.N` (and Phase A–D items as `§5.1-<id>`). The records are kept in git, not in the tree:

- `§4.5.2` – `§4.5.134`: `git show acabe991:docs/history/ROADMAP_ARCHIVE_2026-07-16.md`
- `§5.1-<id>` (Phase A–D): `git show acabe991:docs/history/ROADMAP_ARCHIVE_PHASE_A-D.md`
- `§4.5.135` – `§4.5.582`: `git show acabe991:docs/history/ROADMAP_ARCHIVE.md` (search for `#### 4.5.N `)
- `§4.5.583` and later: the slice's commit message — `git log --grep '§4.5.N'`

Finished behaviour is documented in [docs/manual/](../manual/) and [CHANGELOG.md](../../CHANGELOG.md).
