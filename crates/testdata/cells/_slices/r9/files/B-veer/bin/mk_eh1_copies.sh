#!/bin/bash
B=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/B-veer
cd $B && rm -rf eh1v eh1s && cp -R eh1w eh1v && cp -R eh1w eh1s && rm -rf eh1v/.git eh1s/.git
cd $B/eh1v && for f in $(grep -rlE '`(ifdef|ifndef|elsif) +VERILATOR\b' design testbench); do perl -pi -e 's/`(ifdef|ifndef|elsif)(\s+)VERILATOR\b/`$1$2CENSUS_VERILATOR/g' $f; done
python3 -I - $B/eh1s/testbench/ahb_sif.sv $B/eh1s/testbench/tb_top.sv <<'PY'
import re,sys
p=sys.argv[1]; s=open(p,newline='').read()
# S1: sv2v has no assoc arrays -> fixed [0:'h1ffff] memory indexed by addr[16:0] (program at 0x0-0x1ffff; mailbox writes alias harmlessly)
s=s.replace("bit [7:0] mem [int];","bit [7:0] mem [0:32'h1ffff];  // CENSUS S1")
s=re.sub(r"mem\[(araddr|awaddr)\+(\d)\]", r"mem[17'(\1+\2)]", s)
s=re.sub(r"mem\[(araddr|awaddr)\]", r"mem[17'(\1)]", s)
s=re.sub(r"mem\[\{(\w+)\[31:3\], 3'd(\d)\}\]", r"mem[{\1[16:3], 3'd\2}]", s)
# S2: explicit zero inits that sv2v drops from 2-state types
s=s.replace("initial begin $readmemh(\"program.hex\", cen_img);","initial begin for (int a = 0; a <= 32'h1ffff; a++) begin mem[a] = 8'h00; cen_img[a] = 8'h00; end $readmemh(\"program.hex\", cen_img);")
s=s.replace("bit [63:0] memdata;","bit [63:0] memdata = '0;")
open(p,'w',newline='').write(s)
t=open(sys.argv[2],newline='').read()
t=t.replace("    bit                         core_clk;","    bit                         core_clk = 1'b0; // CENSUS S2")
t=t.replace("    bit        [31:0]           cycleCnt;","    bit        [31:0]           cycleCnt = 0; // CENSUS S2").replace("    int                         commit_count;","    int                         commit_count = 0; // CENSUS S2")
# S3: tb-only bank helpers (callers stubbed by E11) -> removed, sv2v cannot type them
for fn in ['get_dccm_bank','get_iccm_bank']:
    while ('function int '+fn) in t or ('function '+fn) in t:
        a=t.find('function int '+fn)
        if a<0: a=t.find('function '+fn)
        e=t.index('endfunction',a)+len('endfunction')
        t=t[:a]+'// CENSUS S3: '+fn+' removed'+t[e:]
open(sys.argv[2],'w',newline='').write(t)
PY
echo "v_renamed=$(grep -rn CENSUS_VERILATOR $B/eh1v/design $B/eh1v/testbench | wc -l) s1=$(grep -c 'CENSUS S1' $B/eh1s/testbench/ahb_sif.sv) s2=$(grep -c 'CENSUS S2' $B/eh1s/testbench/tb_top.sv)"
