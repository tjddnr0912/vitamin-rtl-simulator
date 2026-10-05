#!/bin/bash
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export POST=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/probe6/vita
R=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/g/r.py
cd /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/p6
OUTDIR=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/p6/out_new python3 $R --iv --s2v cells/*.sv > log_new.txt 2>&1
for g in ab aa c1 c2 c3 c4 c5 c6 c7 c8; do
  OUTDIR=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/p6/p6_$g python3 $R --novl /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/g/$g/*.sv > log_$g.txt 2>&1
done
for g in c1 c2 c3 c4 c5 c6; do
  OUTDIR=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/p6/p6_pl$g python3 $R --novl /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/plan/$g/*.sv > log_pl$g.txt 2>&1
done
OUTDIR=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/p6/p6_plw python3 $R --novl /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/plan/w/*.sv > log_plw.txt 2>&1
