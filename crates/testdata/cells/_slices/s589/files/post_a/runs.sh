#!/bin/bash
# runs.sh <tag> <sepdir> <dir>...: one-CU staged run (vcmp -> velab -> vrun), output <cell>.<tag>
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
TAG=$1; P=$2; shift 2
for d in "$@"; do
  cd "$d" || exit 9
  for f in *.sv; do b=${f%.sv}; { perl -e 'alarm 20; exec @ARGV' $P/vcmp "$f" -o "$b.$TAG.vu" && perl -e 'alarm 20; exec @ARGV' $P/velab "$b.$TAG.vu" -o "$b.$TAG.velab" && perl -e 'alarm 20; exec @ARGV' $P/vrun "$b.$TAG.velab"; echo "rc=$?"; } > "$b.$TAG" 2>&1; rm -f "$b.$TAG.vu" "$b.$TAG.velab"; done
done
