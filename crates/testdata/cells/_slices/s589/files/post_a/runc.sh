#!/bin/bash
# runc.sh <tag> <vita> <dir>...: one-shot run of every *.sv in each dir, 20 s self-alarm, output <cell>.<tag>
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
TAG=$1; V=$2; shift 2
for d in "$@"; do
  cd "$d" || exit 9
  for f in *.sv; do b=${f%.sv}; { perl -e 'alarm 20; exec @ARGV' "$V" "$f"; echo "rc=$?"; } > "$b.$TAG" 2>&1; done
done
