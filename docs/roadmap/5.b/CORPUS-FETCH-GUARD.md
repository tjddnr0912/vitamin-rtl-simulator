(§5.b CORPUS-FETCH-GUARD · OPEN) (`corpus-runner fetch` removes clean clones it misreads as stale):
* `corpus-runner fetch` (`fetch.rs:168` `clear_stale`, §4.5.603) judges a clone stale from a file parse of `.git/HEAD`: a reftable clone (`refs/heads/.invalid`), a `ref:` chain past one level and a worktree's gitfile read stale, so a clean clone at the pin is removed and re-cloned (fix: when the parse fails, `.git` is a file or `.git/reftable` exists, ask `git --git-dir <d>/.git rev-parse --verify --quiet HEAD^{commit}`)
* symptom: a stale clone at a local, unpushed commit with a clean tree is deleted (refuse when `git --git-dir <d>/.git rev-list -n 1 HEAD --branches --not --remotes` prints)
* fix: a removal should rename the clone aside first (`src.removing-<pid>`), re-check and rename back on refusal
* symptom: an untracked `.DS_Store` makes a stale clone unremovable (exit 1)
* source: bundle 1's review round 3 (R3-N1–N4)
* status: OPEN
