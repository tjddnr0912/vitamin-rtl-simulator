#!/bin/sh
# usage: ovl.sh <files.txt>  -> prints list with patched overlay substituted
. /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/A-opentitan/bin/env.sh
while read f; do p=$R/patched/${f#$OT/}; if [ -f "$p" ]; then echo $p; else echo $f; fi; done < $1
