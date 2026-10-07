#!/bin/sh
# usage: run_tb.sh <name> [steps]  steps: deps vita vlo vlp sv  (tb file = R/tb/tb_<name>.sv, top = tb)
. /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/A-opentitan/bin/env.sh
name=$1; steps=${2:-"deps vita vlo vlp sv"}; tbf=$R/tb/tb_$name.sv
d=$R/ips/$name/tb; mkdir -p $d; cd $d
W="perl $S/s2-review/sound/p3/wd.pl"
VLF="--binary --timing --timescale 1ns/1ns -Wno-fatal -Wno-lint -Wno-style -Wno-MULTIDRIVEN --x-assign unique --x-initial unique --top-module tb -j 4"
for s in $steps; do case $s in
deps) python3 -I $R/bin/deps.py $OT tb $tbf > files.txt 2> deps.err; $R/bin/ovl.sh files.txt > files.p.txt; echo "deps: $(wc -l < files.txt) $(cat deps.err)";;
vita) $W 300 6000000 vita.log $VITA --top tb $DEFS -I$R/tb -I$R/patched/hw/ip/prim/rtl -I$R/patched/hw/dv/sv/dv_utils $(cat files.p.txt) > vita.wd; echo "vita: $(cat vita.wd) $(grep -E '^errors=' vita.log) $(grep -E '^DIGEST|simulation ended' vita.log | tr '\n' ' ')";;
vlo|vlp) if [ $s = vlo ]; then fl=files.txt; I="$INCS"; else fl=files.p.txt; I="-I$R/patched/hw/ip/prim/rtl -I$R/patched/hw/dv/sv/dv_utils"; fi
  if ! $R/bin/dfcheck.sh; then echo "$s: SKIPPED disk low"; echo "DISK_LOW $(date) $name $s" >> $R/REPORT.md; continue; fi
  rm -rf obj_$s; $R/bin/lock.sh $W 1200 10000000 $s.build.log /opt/homebrew/bin/verilator $VLF -Mdir obj_$s -o v $DEFS -I$R/tb $I $(cat $fl) > $s.build.wd 2>&1
  echo "$s build: $(cat $s.build.wd) $(grep -c '^%Error' $s.build.log) errors"
  if [ -x obj_$s/v ]; then
    if [ $s = vlo ]; then seeds="1:0 2:1 3:2 4:2 5:2"; else seeds="1:2"; fi
    for sr in $seeds; do sd=${sr%:*}; rr=${sr#*:}; $W 300 6000000 $s.run.$sd.log obj_$s/v +verilator+seed+$sd +verilator+rand+reset+$rr > $s.run.$sd.wd; echo "$s seed=$sd reset=$rr: $(cat $s.run.$sd.wd) $(grep -E '^DIGEST|WATCHDOG' $s.run.$sd.log | tr '\n' ' ')"; done
  fi
  rm -rf obj_$s;;
sv) $W 300 6000000 sv2v.log $SV2V $DEFS -I$R/tb $INCS $(cat files.txt) -w sv2v_out.v > sv.wd; echo "sv2v: $(cat sv.wd)"; if [ -s sv2v_out.v ]; then $W 300 6000000 sviv.log sh -c "/opt/homebrew/bin/iverilog -g2012 -s tb -o sviv.vvp sv2v_out.v && /opt/homebrew/bin/vvp -n sviv.vvp" > sviv.wd; echo "sv2v->iv: $(cat sviv.wd) $(grep -E '^DIGEST|WATCHDOG' sviv.log) $(grep -m1 -i error sviv.log)"; rm -f sviv.vvp; fi;;
esac; done
