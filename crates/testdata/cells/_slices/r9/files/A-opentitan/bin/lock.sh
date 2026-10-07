#!/bin/sh
# usage: lock.sh <cmd...>  -- holds S/row9/.verilator.lock while cmd runs; always releases
L=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/.verilator.lock
n=0
until mkdir "$L" 2>/dev/null; do sleep 30; n=$((n+1)); if [ $n -gt 120 ]; then echo "LOCK_TIMEOUT"; exit 99; fi; done
trap 'rmdir "$L" 2>/dev/null' EXIT INT TERM HUP
"$@"
rc=$?
rmdir "$L" 2>/dev/null
trap - EXIT
exit $rc
