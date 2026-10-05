#!/bin/bash
# usage: lanes.sh pre|post   -> S/lanes/<side>/<cell>/{out,err,rc}
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s586
SIDE=$1
B=$S/$SIDE
G=$S/g
O=$S/lanes/$SIDE
rm -rf $O; mkdir -p $O
EX=/Users/seongwookjang/project/git/wt-s586/examples

# cell <name> <srcdir> <cmd...> : copy sources of srcdir into O/name, run cmd there
cell() {
  local name=$1 src=$2; shift 2
  local d=$O/$name
  mkdir -p $d
  for f in $(ls $src | grep -E '\.(sv|svh)$'); do cp $src/$f $d/; done
  ( cd $d && "$@" > out 2> err; echo $? > rc )
}
# step <name> <cmd...> : run in an existing cell dir, outputs <step>.{out,err,rc}
step() {
  local name=$1 st=$2; shift 2
  ( cd $O/$name && "$@" > $st.out 2> $st.err; echo $? > $st.rc )
}

V=$B/vita
cell native  $G/c41_lanes $V --backend native t.sv
cell interp  $G/c41_lanes $V --backend interp t.sv
cell vm      $G/c41_lanes $V --backend vm t.sv
cell q       $G/c41_lanes $V -q t.sv
cell wno     $G/c41_lanes $V -Wno-I2021 t.sv
cell obs     $G/c41_lanes $V --obs-dir obs t.sv
cell mc      $G/c41_lanes $V vcmp -o t.vu t.sv
step mc velab $V velab -o t.velab t.vu
step mc vrun  $V vrun t.velab
cell sep     $G/c48_lanes2 $B/sep/vcmp -o t.vu t.sv
step sep velab $B/sep/velab -o t.velab t.vu
step sep vrun  $B/sep/vrun t.velab
cell def48   $G/c48_lanes2 $V t.sv
if [ $SIDE != pre ]; then
  cell jit   $G/c48_lanes2 env VITA_JIT=1 $B/vita_jit t.sv
  cell prod  $G/c48_lanes2 $B/vita_product t.sv
  cell prodi $G/c48_lanes2 $B/vita_product --backend interp t.sv
fi
cell log     $G/c53_log $V --log run.log t.sv
cell wl      $G/c50_worklib $B/sep/vcmp --work L=lib/L a.sv
step wl vcmp_b $B/sep/vcmp --work L=lib/L b.sv
step wl velab  $B/sep/velab -L L=lib/L --top t -o x.velab
step wl vrun   $B/sep/vrun x.velab
cell c13b    $G/c13b_uninst_top $V --top t t.sv
cell c14     $G/c14_gen_untaken $V t.sv
cell c17     $G/c17_uncalled_fn $V t.sv
cell c42     $G/c42_prio_then_unique $V t.sv
cell c44     $G/c44_parse_err $V t.sv
cell c45     $G/c45_ifdef_out $V t.sv
cell c45d    $G/c45_ifdef_out $V -DNOPE t.sv
cell c46     $G/c46_macro $V t.sv
cell c47     $G/c47_timescale_werror $V t.sv
cell c47w    $G/c47_timescale_werror $V -Werror t.sv
cell c47w1   $G/c47_timescale_werror $V -Werror=I2021 t.sv
cell c52     $G/c52_include $V t.sv
cell c43ab   $G/c43_multifile $V a.sv b.sv
cell c43ba   $G/c43_multifile $V b.sv a.sv
cell c51u    $G/c51_constraint_unique $V u.sv
cell c51c    $G/c51_constraint_unique $V t.sv
cell c22     $G/c22_priority_only_noovl $V t.sv
for e in 000_counter 001_alu 002_traffic_fsm 003_shift_register; do
  mkdir -p $O/ex_$e; cp $EX/$e.sv $O/ex_$e/
  ( cd $O/ex_$e && $V $e.sv > out 2> err; echo $? > rc )
done
R2=$S/r2/cells
cell r2c02   $R2/c02 $V t.sv
cell r2c02v  $R2/c02 $V vcmp -o t.vu t.sv
cell r2c05   $R2/c05 $V t.sv
cell r2c01e  $R2/c01e $V t.sv
echo done > $O/DONE
