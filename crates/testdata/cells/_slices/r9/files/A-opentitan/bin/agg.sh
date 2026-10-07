#!/bin/sh
# usage: agg.sh <vita.log>  -> distinct error messages (instance path stripped) with counts and first site
. /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/A-opentitan/bin/env.sh
grep -E "(error|fatal)\[VITA" $1 | sed "s#$OT/##g; s#$R/patched/#P:#g" | sed -E 's/ \[in [^]]*\]$//' | awk '{site=$1; $1=""; msg=$0; c[msg]++; if(!(msg in f)) f[msg]=site} END {for (m in c) printf "%4d %s |%s\n", c[m], f[m], m}' | sort -rn
