#!/bin/sh
# usage: vita.sh <outlog> <vita args...>   (300 s / 6 GB watchdog)
. /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/A-opentitan/bin/env.sh
out=$1; shift
pgrep -fl target/debug/vita >/dev/null 2>&1
perl $S/s2-review/sound/p3/wd.pl 300 6000000 $out $VITA "$@"
