#!/bin/bash
# regenerate el2v (verilator: VERILATOR ifdefs renamed) and el2s (sv2v: fixed-size tb memory) from el2w
B=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/B-veer
cd $B && rm -rf el2v el2s && cp -R el2w el2v && cp -R el2w el2s && rm -rf el2v/.git el2s/.git
cd $B/el2v && for f in $(grep -rlE '`(ifdef|ifndef|elsif) +VERILATOR\b' design testbench); do perl -pi -e 's/`(ifdef|ifndef|elsif)(\s+)VERILATOR\b/`$1$2CENSUS_VERILATOR/g' $f; done
python3 -I - $B/el2s/testbench/ahb_sif.sv <<'PY'
import re,sys
p=sys.argv[1]; s=open(p,newline='').read()
s=s.replace("bit [7:0] mem[int];","bit [7:0] mem [0:32'h1ffff];  // CENSUS S1: sv2v has no assoc arrays; index = {addr[30], addr[15:0]}")
s=re.sub(r"mem\[\{(\w+)\[31:3\], 3'd(\d)\}\]", r"mem[{\1[30], \1[15:3], 3'd\2}]", s)
s=s.replace("mem[32'h80000000 + a] = cen_img[a];","mem[a] = cen_img[a];")
# S2: sv2v turns `bit` into a 4-state reg without its implicit 0: make the 0 explicit (a no-op for 2-state `bit`)
s=s.replace("  bit [7:0] wscnt;","  bit [7:0] wscnt = '0;").replace("  bit dws_rand;","  bit dws_rand = 1'b0;").replace("  bit iws_rand;","  bit iws_rand = 1'b0;").replace("  bit ok;","  bit ok = 1'b0;")
s=s.replace('initial begin $readmemh("program_rebased.hex", cen_img);','initial begin for (int a = 0; a <= 32\'h1ffff; a++) mem[a] = 8\'h00; for (int a = 0; a <= 32\'hffff; a++) cen_img[a] = 8\'h00; $readmemh("program_rebased.hex", cen_img);')
open(p,'w',newline='').write(s)
PY
perl -pi -e "s/^    bit                         core_clk;/    bit                         core_clk = 1\x27b0; \/\/ CENSUS S2/" $B/el2s/testbench/tb_top.sv
echo "v_renamed=$(grep -rn CENSUS_VERILATOR $B/el2v/design $B/el2v/testbench | wc -l) s_mem=$(grep -c 'CENSUS S1' $B/el2s/testbench/ahb_sif.sv) s2_clk=$(grep -c 'CENSUS S2' $B/el2s/testbench/tb_top.sv) s2_init=$(grep -c "mem\[a\] = 8.h00" $B/el2s/testbench/ahb_sif.sv)"
