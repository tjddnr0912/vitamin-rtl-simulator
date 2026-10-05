#!/bin/bash
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s586; D=$S/r1/diff; C=$D/cells
v() { n=$1; dir=$2; shift 2; o=$D/v/$n; mkdir -p $o
  for b in pre post; do B=$S/$b/vita; ( cd $dir; $B "${@//@B@/$b}" > $o/$b.out 2> $o/$b.err; echo $? > $o/$b.rc ); done
  printf '%-12s rc pre=%s post=%s stdout=%s I2021_pre=%s I2021_post=%s\n' $n $(cat $o/pre.rc) $(cat $o/post.rc) $(cmp -s $o/pre.out $o/post.out && echo SAME || echo DIFF) $(grep -c 'VITA-I2021' $o/pre.err) $(grep -c 'VITA-I2021' $o/post.err); }
iv() { n=$1; shift; o=$D/iv/$n; mkdir -p $o; ( cd $C; iverilog -g2012 -o $o/a.vvp "$@" > $o/cout 2>&1; echo $? > $o/crc; [ -f $o/a.vvp ] && vvp -n $o/a.vvp > $o/out 2>&1 ); }
for f in b01_final b02_fork b03_class b04_iface b05_program b06_pkg b07_genfor b08_casegen b09_attr b10_label b11_inside b12_nested b13_nonqual b14_paste b15_line b16_tabs c02_fatal c03_w2004 e02_uif e04_nomatch e03_iv_sites; do v $f $C $f.sv; iv $f $f.sv; done
v c01 $C --log $D/v/c01/@B@.log --obs-dir $D/v/c01/@B@.obs c01_elabfail.sv
v c02q $C -q c02_fatal.sv
v e01 $C/e01 m.sv; iv e01 e01/m.sv
v e01wno1 $C/e01 -Wno-I-PARSE-UNIQUE-OVERLAP-UNCHECKED m.sv
v e01wno2 $C/e01 -Wno-VITA-I2021 m.sv
v a01 $C -f a01_outer.f; iv a01 a01_x.sv a01_y.sv
v a01dump $C -f a01_outer.f --dump-filelist
v a02dup $C a01_x.sv a01_y.sv a01_y.sv
iv e03s -s top e03_iv_sites.sv
