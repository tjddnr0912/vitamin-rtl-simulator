#!/bin/bash
# run.sh <base> : runs PRE/POST vita on <base>_u.sv/_p.sv, verilator on both, iverilog on _p
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s587
b=$1; cd $S/r2/sound/c
for v in u p; do for t in pre post_c; do
  rm -rf obs_${b}_${v}_$t; out=$($S/$t/vita --obs-dir obs_${b}_${v}_$t ${b}_$v.sv 2>&1); rc=$?
  echo "$out" > ${b}_${v}.$t.txt
  echo "[$b $v vita-$t rc=$rc] $(echo "$out" | grep -E "P=|b=|v=|W=|B=|y=|VITA-[EW][0-9]" | grep -v '^$' | head -6 | tr '\n' '|' | cut -c1-600)"
done
  rm -rf vl_${b}_$v; if verilator --binary --timing --assert -Wno-fatal --Mdir vl_${b}_$v -o sim ${b}_$v.sv >/dev/null 2>vl_${b}_$v.build; then o=$(./vl_${b}_$v/sim +verilator+error+limit+1000 2>&1 | grep -v '^- \|^$' | head -6 | tr '\n' '|'); else o="BUILD_FAIL $(grep -E '%Error' vl_${b}_$v.build | head -3 | tr '\n' '|')"; fi
  echo "[$b $v verilator] $o" | cut -c1-600; rm -rf vl_${b}_$v
done
if iverilog -g2012 -o ${b}_p.vvp ${b}_p.sv 2>${b}_p.ivlerr; then o=$(vvp -n ${b}_p.vvp 2>&1 | head -5 | tr '\n' '|'); else o="IVL_FAIL $(head -3 ${b}_p.ivlerr | tr '\n' '|')"; fi
echo "[$b p iverilog] $o" | cut -c1-500; rm -f ${b}_p.vvp
