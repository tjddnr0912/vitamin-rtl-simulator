(§5.2 Q-VMSELSHIFT · rank ① · source Q-RELDELAY r4 lens-sound P3, r5 design review d6) (debug build panics on a select read partly below the vector):
* slot: after Q-IDXVALUE
* in a debug build, the native and VM select read panics on a read partly below the vector: `op_select` (`native_eval/exec_vm.rs:762` `sv >> si`) already tests `si >= 0`, but the offset reaches it as its residue modulo 2^32, so a read partly below the vector gets `si = 2^32 + (l + i)`, which passes `si >= 0` and `(si as u32) < src_w` and shifts by 64 or more (`attempt to shift right with overflow`)
* a release build is right only because Rust masks the shift amount (2^32 + k ≡ k mod 64): `logic [0:0] p; integer k = -1; rd2 = p[k +: 2];` panics (exit 101) on native and vm, interp prints `rd2=1x`, release `1x` on every backend = iverilog = sv2v
* a PRE debug build (d213ea53) panics the same on the plain forms and PRE's literal member forms (row 24 r4 lens cells c6/pl_assign `assign y = p[8:7]`, c6/pl_comb_arr `always_comb y = arr[1][9:6]`, c6/pl_ip_neg `v[i +: 4]` i = -2, c5/ps.comb, c5/ps.ff), while a slice of a signal base (`v[9:6]` in an initializer or `always_comb`) does not reach it
* the same family at `sim-engine` `binops.rs:722` `let src_idx = lsb + i` (attempt to add with overflow), debug only, PRE 719e37d2's debug build, release right: bare `logic [3:0][-6:-1] p; longint k = 64'sh8000000000000000; p[1][k -: 2]` exits 101 (W/qpkrdlsb/r1/lens-sound/s7d/v2), its array twin does not
* on an element of an array `logic [3:0][3:-2] q [2]; logic [63:0] k = 64'h7FFF_FFFF_FFFF_FFFE; q[1][1][k +: 3]` exits 101, release `xxx` = hand, its bare twin does not (W/qpkrdlsb/r1/lens-diff/d3p c2 c3)
* first: find where the offset loses its sign on the way to the select
* pre-existing
