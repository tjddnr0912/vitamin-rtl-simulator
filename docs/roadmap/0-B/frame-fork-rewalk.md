(§0-B frame-fork-rewalk) (fork inside a frame: re-walk, nested forking task, sibling visibility):
* symptom: fork-in-frame: `fork_arms_self_contained`'s duplicate re-walk
* symptom: an elaborate reject for a fork arm calling a forking task (F4004 guards it)
* symptom: zero-delay sibling visibility not differentially verified.
