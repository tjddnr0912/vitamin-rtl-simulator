#!/bin/bash
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export POST=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/post_dev/vita
R=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/g/r.py
cd /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/p6
for g in ab aa c1 c2 c3 c4 c5 c6 c7 c8; do
  OUTDIR=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/p6/dev_$g python3 $R --novl /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/g/$g/*.sv > logd_$g.txt 2>&1
done
for g in c1 c2 c3 c4 c5 c6 w; do
  OUTDIR=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/p6/dev_pl$g python3 $R --novl /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/plan/$g/*.sv > logd_pl$g.txt 2>&1
done
OUTDIR=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s590/p6/dev_new python3 $R --novl cells/*.sv > logd_new.txt 2>&1
