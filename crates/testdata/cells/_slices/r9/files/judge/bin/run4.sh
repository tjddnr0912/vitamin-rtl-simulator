#!/bin/bash
# usage: run4.sh <id> <src.sv> [top]
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad
WD="perl $S/s2-review/sound/p3/wd.pl"
VITA=$S/accept/vita-final
SV2V=$S/blog4/sv2v/sv2v-macOS/sv2v
id=$1; src=$2; top=$3
d=$S/row9/judge/runs/$id; mkdir -p $d; cd $d || exit 9
ext="${src##*.}"; cp "$src" t.$ext; f=t.$ext; cp "$(dirname "$src")"/*.hex . 2>/dev/null
rm -f vita.out iv.c.out iv.out s2v.c.out s2v.out vl.c.out vl.out sv.v
$WD 120 4000000 vita.out $VITA $f >> vita.out
$WD 120 4000000 iv.c.out iverilog -g2012 -o iv.vvp $f >> iv.c.out
if [ -f iv.vvp ]; then $WD 120 4000000 iv.out vvp -n iv.vvp >> iv.out; rm -f iv.vvp; fi
$WD 120 4000000 s2v.c.out sh -c "$SV2V $f > sv.v" >> s2v.c.out
if [ -s sv.v ]; then $WD 120 4000000 s2v.c2.out iverilog -g2012 -o sv.vvp sv.v >> s2v.c2.out; cat s2v.c2.out >> s2v.c.out; rm -f s2v.c2.out
  if [ -f sv.vvp ]; then $WD 120 4000000 s2v.out vvp -n sv.vvp >> s2v.out; rm -f sv.vvp; fi; fi
n=0; until mkdir $S/row9/.verilator.lock 2>/dev/null; do n=$((n+1)); [ $n -gt 900 ] && { echo "LOCK TIMEOUT" > vl.c.out; exit 0; }; sleep 1; done
topflag=""; [ -n "$top" ] && topflag="--top-module $top"
$WD 120 4000000 vl.c.out verilator --binary -Wno-fatal -Wno-lint -Wno-style $topflag -Mdir obj $f >> vl.c.out
exe=$(ls obj/V* 2>/dev/null | grep -v '\.' | head -1)
if [ -n "$exe" ] && [ -x "$exe" ]; then $WD 120 4000000 vl.out $exe >> vl.out; fi
rm -rf obj; rmdir $S/row9/.verilator.lock
