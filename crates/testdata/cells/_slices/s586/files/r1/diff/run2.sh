#!/bin/bash
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s586; D=$S/r1/diff; C=$D/cells; N=$C/r2
v() { n=$1; dir=$2; shift 2; o=$D/v2/$n; mkdir -p $o
  for b in pre post2; do B=$S/$b/vita; ( cd $dir; $B "${@//@B@/$b}" > $o/$b.out 2> $o/$b.err; echo $? > $o/$b.rc ); done
  p=$D/v/$n; same() { cmp -s "$1" "$2" && echo S || echo D; }
  printf '%-12s rc=%s/%s stdoutPRE~P2=%s I2021=%s | P2vsPOST out=%s err=%s rc=%s | PREvsPRE1 err=%s\n' $n $(cat $o/pre.rc) $(cat $o/post2.rc) $(same $o/pre.out $o/post2.out) $(grep -c 'VITA-I2021' $o/post2.err) $(same $o/post2.out $p/post.out) $(same <(sed 's/post2/post/g' $o/post2.err) $p/post.err) $(same $o/post2.rc $p/post.rc) $(same $o/pre.err $p/pre.err); }
for f in b01_final b02_fork b03_class b04_iface b05_program b06_pkg b07_genfor b08_casegen b09_attr b10_label b11_inside b12_nested b13_nonqual b14_paste b15_line b16_tabs c02_fatal c03_w2004 e02_uif e04_nomatch e03_iv_sites; do v $f $C $f.sv; done
v c01 $C --log $D/v2/c01/@B@.log --obs-dir $D/v2/c01/@B@.obs c01_elabfail.sv; cmp <(sed 's/post2/post/g' $D/v2/c01/post2.log) $D/v/c01/post.log && echo "c01 log P2==POST"
v c02q $C -q c02_fatal.sv; v e01 $C/e01 m.sv; v e01wno1 $C/e01 -Wno-I-PARSE-UNIQUE-OVERLAP-UNCHECKED m.sv; v e01wno2 $C/e01 -Wno-VITA-I2021 m.sv
v a01 $C -f a01_outer.f; v a01dump $C -f a01_outer.f --dump-filelist; v a02dup $C a01_x.sv a01_y.sv a01_y.sv
cd $C; for be in native interp vm; do echo "c04 $be: $($S/post2/vita --backend $be c04_dup.sv 2>/dev/null | head -1)"; done; echo "c04 jit: $(VITA_JIT=1 $S/post2/vita_jit c04_dup.sv 2>/dev/null | head -1) / product: $($S/post2/vita_product c04_dup.sv 2>/dev/null | head -1)"
o=$D/v2/stg; mkdir -p $o; $S/post2/sep/vcmp -o $o/p2.vu c04_dup.sv 2> $o/vcmp.err; echo "sep vcmp rc=$? I2021=$(grep -c VITA-I2021 $o/vcmp.err)"; cmp $o/p2.vu $D/v/c04/pre.vu && echo "vu P2==PRE"; $S/post2/sep/velab -o $o/p2.velab $o/p2.vu 2> $o/velab.err; echo "sep velab rc=$? I2021=$(grep -c VITA-I2021 $o/velab.err)"; cmp $o/p2.velab $D/v/c04/pre.velab && echo "velab P2==PRE"; $S/post2/sep/vrun $o/p2.velab > $o/vrun.out 2> $o/vrun.err; echo "sep vrun rc=$? I2021=$(grep -c VITA-I2021 $o/vrun.err) $(head -1 $o/vrun.out)"
for fl in -Werror -Werror=W2004; do $S/post2/vita $fl c03_w2004.sv > $o/w.out 2> $o/w.err; echo "c03 $fl rc=$? $(sed 's/post2/post/' $o/w.err | cmp -s - $D/v/werr/post$fl.err && echo errSAMEasPOST || echo errDIFF)"; done
echo "#### new cells (stderr PRE / POST / POST2, one-shot then vcmp)"
cd $N; for f in n01_cu_func n02_pkg_only n03_class_only n04_typedef_func n05_lp_task n06_iface_only n07_ifdef_empty n09_cu_plus_mod n10_pp_err n11_lex_str n13_lex_byte n12_priority0; do o=$D/v2/$f; mkdir -p $o
  for b in pre post post2; do $S/$b/vita $f.sv > $o/$b.out 2> $o/$b.err; echo $? > $o/$b.rc; $S/$b/vita vcmp -o $o/$b.vu $f.sv > /dev/null 2> $o/$b.cerr; echo $? > $o/$b.crc; done
  echo "## $f rc pre/post/post2=$(cat $o/pre.rc)/$(cat $o/post.rc)/$(cat $o/post2.rc) vcmp=$(cat $o/pre.crc)/$(cat $o/post.crc)/$(cat $o/post2.crc) errPRE==P2:$(cmp -s $o/pre.err $o/post2.err && echo Y || echo N) vcmpPRE==P2:$(cmp -s $o/pre.cerr $o/post2.cerr && echo Y || echo N) POST==P2:$(cmp -s $o/post.err $o/post2.err && echo Y || echo N)"
  echo "   POST2: $(sed -E 's/(UNCHECKED: ).*/\1<msg>/' $o/post2.err | cut -c1-150 | tr '\n' '|')"; cmp -s $o/post.err $o/post2.err || echo "   POST : $(sed -E 's/(UNCHECKED: ).*/\1<msg>/' $o/post.err | cut -c1-150 | tr '\n' '|')"
  echo "   vcmpP2: $(sed -E 's/(UNCHECKED: ).*/\1<msg>/' $o/post2.cerr | cut -c1-120 | tr '\n' '|')"
  iverilog -g2012 -o $o/iv.vvp $f.sv > $o/iv.cout 2>&1; echo "   iverilog crc=$? $(head -3 $o/iv.cout | cut -c1-110 | tr '\n' '|')"; done
$S/post2/vita -DNEVER_DEFINED n07_ifdef_empty.sv > /dev/null 2> $D/v2/n07_ifdef_empty/def.err; echo "n07 -DNEVER_DEFINED rc=$? I2021=$(grep -c VITA-I2021 $D/v2/n07_ifdef_empty/def.err)"
