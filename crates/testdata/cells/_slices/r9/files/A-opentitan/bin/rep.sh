#!/bin/sh
# usage: rep.sh <file.sv> [top=t]   runs vita, iverilog, sv2v->iverilog, verilator --binary; prints outputs
. /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/A-opentitan/bin/env.sh
f=$(cd $(dirname $1) && pwd)/$(basename $1); top=${2:-t}; b=$(basename $f .sv); d=$(dirname $f)/out_$b; mkdir -p $d; cd $d
W="perl $S/s2-review/sound/p3/wd.pl"
$W 60 6000000 vita.out $VITA --top $top $f > vita.wd; echo "--- vita $(cat vita.wd)"; grep -v "^errors=0 warnings=0" vita.out | head -${N:-12}
$W 60 6000000 iv.out sh -c "/opt/homebrew/bin/iverilog -g2012 -s $top -o iv.vvp $f && /opt/homebrew/bin/vvp -n iv.vvp" > iv.wd; echo "--- iverilog $(cat iv.wd)"; grep -v "VCD info\|^\$finish\|finish called" iv.out | head -${N:-12}
$W 60 6000000 sv.out sh -c "$SV2V $f > sv.v && /opt/homebrew/bin/iverilog -g2012 -s $top -o sv.vvp sv.v && /opt/homebrew/bin/vvp -n sv.vvp" > sv.wd; echo "--- sv2v->iverilog $(cat sv.wd)"; grep -v "VCD info\|finish called" sv.out | head -${N:-12}
if [ -z "$NOVL" ]; then
if ! $R/bin/dfcheck.sh; then echo "--- verilator SKIPPED disk low"; echo "DISK_LOW at $(date) building $f" >> $R/REPORT.md; exit 0; fi
rm -rf vobj; $R/bin/lock.sh $W 1200 10000000 vlb.out /opt/homebrew/bin/verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style --x-assign unique --x-initial unique --top-module $top -Mdir vobj -o v $f > vlb.wd; 
if [ -x vobj/v ]; then $W 300 6000000 vl.out vobj/v +verilator+seed+7 +verilator+rand+reset+2 > vl.wd; echo "--- verilator $(cat vl.wd)"; grep -v "^- \|Verilator: \$finish\|^- Verilator" vl.out | head -${N:-12}; else echo "--- verilator BUILD $(cat vlb.wd)"; grep "%Error" vlb.out | head -5; fi
rm -rf vobj
fi
