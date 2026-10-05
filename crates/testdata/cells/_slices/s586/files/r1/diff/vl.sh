#!/bin/bash
D=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s586/r1/diff
C=$D/cells; cd $C
one() { n=$1; top=$2; shift 2; o=$D/vl/$n; mkdir -p $o
  verilator --binary --timing --assert -Wno-fatal -j 0 --top-module $top -Mdir $o/obj "$@" > $o/cout 2>&1; echo $? > $o/crc
  if [ -x $o/obj/V$top ]; then $o/obj/V$top +verilator+error+limit+1000 > $o/out 2>&1; echo $? > $o/rc; else echo NOBIN > $o/rc; fi; }
for f in b01_final b02_fork b03_class b04_iface b05_program b06_pkg b07_genfor b08_casegen b09_attr b10_label b11_inside b12_nested b13_nonqual b14_paste b15_line b16_tabs c02_fatal c03_w2004 e02_uif e03_iv_sites e04_nomatch; do one $f top $f.sv; done
one e01 m e01/m.sv
one a01 top a01_x.sv a01_y.sv
echo 0 > $D/vl/done.rc
