(§5.2 Q-SELNORM · rank ① · source Q-PKRDLSB re-plan W/qpkrdlsb/replan/REPLAN.txt §11, r1 lens-sound F-A, F-B) (select-index normalization: unsealed narrow calls and bounds clamped at i32):
* slot: past the last numbered slot
* slot: starts only on a track whose next C row cannot start
* slot: `packed.rs`: D12 file check at start
* one funnel, the normalization of a select index (`elaborate/src/packed.rs`), two clauses Q-PKRDLSB's re-plan raised, no page or corpus line (all 77 decls innermost LSB 0, REPLAN §11): Q-SEALCALL — a narrow unsigned index the seal cannot prove is left in the 32-bit context (32 bits and more have no such context): `sealed_signed_index` (`packed.rs:1398`) returns None when `expr_provably_unsigned` is false, which it is for a call, a shaped hierarchical name and an array reduction, though `canonical_self_width` calls them unsigned, so `norm_sub_k` emits `Sub(raw, k)` and a 4-bit `f(a) + f(b)` adds at 32 bits: `logic [3:0][31:28] p; p[1][(f(a) + f(b)) +: 2]` a = b = 15 `R a 11` on PRE 719e37d2 = sv2v = verilator, iverilog `R a xx` = hand (self-determined 14; W/qpkrdlsb/r2/lens-diff/d4w/sealcall.sv), bare twin of W/qpkrdlsb/r1/lens-sound s1a, s1u a01 a06 a07 a09 u01-u17
* note: a bare narrow call or name reads right there, so width alone cannot tell it from a composite (48 `fn(k)` rows of family e, h_nar_m7, W/qpkrdlsb/r2/EXPECT-r2.tsv)
* note: §2 "Call index unsealed" is the same seal
* scope: Q-BOUNDI32 — a declared bound past `i32` is clamped where it is subtracted: `norm_offset_for_range` (`packed.rs:1657`) `lo.min(i32::MAX)`, `hi.clamp(..)`, the net twin (`norm_offset_for_net`, `:1564`) the same, so `logic [3:0][2147483651:2147483648] p; p[1][2147483647 +: 1]` `R a 0` on PRE 719e37d2 = iverilog = sv2v, verilator `R a 1`, hand x (W/qpkrdlsb/r2/lens-diff/d4w/boundi32.sv; r1 lens-sound z01-z05, f2_big, f2_clamp)
* note: reach in real RTL ~0
* status: both pre-existing
* note: Q-PKRDLSB's re-derivation needs no E1 once Q-SEALCALL seals what `canonical_self_width` calls unsigned, no E4 on the leaf once Q-BOUNDI32 carries a bound past `i32`
* source: r2 lens-diff D4
