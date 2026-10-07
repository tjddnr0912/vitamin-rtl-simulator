#!/bin/bash
# usage: s2v_build.sh <outdir> <extra -D...>   (from el2s; patches the sv2v output for iverilog; adds an end-time watchdog)
B=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/B-veer
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad
O=$1; shift; mkdir -p $O; cd $O; W=$B/el2s
sed "s#$B/el2w/#$B/el2s/#; s#^snapshots/#$B/work-el2/snapshots/#" $B/work-el2/files-ahb-w.txt > files-s.txt
perl $S/s2-review/sound/p3/wd.pl 300 6000000 sv2v.log $S/blog4/sv2v/sv2v-macOS/sv2v -E Always -DRV_OPENSOURCE "$@" -I$B/work-el2/snapshots/ahb -I$W/testbench -I$W/design/include -I$W/design/lib --top tb_top -w el2.v $(cat files-s.txt)
sed -i '' 's/(\* full_case, parallel_case \*)//' el2.v
python3 -I - el2.v <<'PY'
import re,sys
p=sys.argv[1]; s=open(p).read(); n={}
s,n['dasm_fn']=re.subn(r"\tfunction static string dasm\w*;.*?\tendfunction\n","",s,flags=re.S)
s,n['dasm_call']=re.subn(r"cen_s4 = dasm\([^;]*\);",'cen_s4 = "";',s)
for t in ['dump_signature','slam_dccm_ram','slam_iccm_ram']:
    s,k=re.subn(r"(\ttask "+t+r";\n).*?(\tendtask\n)", r"\1\t\tbegin end\n\2", s, flags=re.S); n[t]=k
s,n['getbank']=re.subn(r"\tfunction signed \[31:0\] get_[di]ccm_bank;.*?\tendfunction\n","",s,flags=re.S)
s,n['cast']=re.subn(r"\[pt\.DCCM_FDATA_WIDTH - 1:0\]","[38:0]",s)
i=s.index('module tb_top'); e=s.index('endmodule',i)
s=s[:e]+'\tinteger census_end_t;\n\tinitial begin : census_end\n\t\tif (!$value$plusargs("census_end=%d", census_end_t)) census_end_t = 100000;\n\t\t#(census_end_t); $display("CENSUS_END t=%0t", $time); $finish;\n\tend\n'+s[e:]
open(p,'w').write(s); print(n)
PY
cp $B/work-el2/program_rebased.hex .
perl $S/s2-review/sound/p3/wd.pl 300 6000000 ivc.log /opt/homebrew/bin/iverilog -g2012 -s tb_top -o el2.vvp el2.v
grep -v "sorry: constant selects\|warning: Synthesis\|vvp.tgt sorry" ivc.log | head -5
ls -la el2.vvp | awk '{print $5, $9}'
