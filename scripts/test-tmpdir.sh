#!/bin/sh
# Test-process runner: give each test process a private, fresh TMPDIR.
#
# nextest runs every test in its own process. Test temp names are
# `vita_<tag>_<pid>_<per-process counter>`, so the counter restarts at 0 in
# each process and a reused PID lands on a stale directory a dead process left
# behind. A per-process TMPDIR makes those names unique by construction, and
# removing it afterwards keeps the run from filling the disk.
#
# Wired as the target runner (CARGO_TARGET_<TRIPLE>_RUNNER) by ci.yml.
# macOS's TMPDIR ends in `/`: strip it, or the path holds a `//`, which a
# `-f` filelist reads as the start of a line comment.
base=${TMPDIR:-/tmp}
while [ "${base%/}" != "$base" ]; do base=${base%/}; done
d=$(mktemp -d "${base:-/tmp}/vita-test.XXXXXX") || exit 1
TMPDIR=$d "$@"
rc=$?
rm -rf "$d"
exit $rc
