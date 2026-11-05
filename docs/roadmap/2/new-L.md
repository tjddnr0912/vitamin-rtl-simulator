(§2 🆕 L · LOUD · in Start-order rows) (loud constant and import residues):
* loud constant / import residues (sub-ids cited from source)
* status: LOUD
  * ⓑ string `$bits` 16 (= LRM): keep · ⓒ `Q * 2` over a real `Q` · ⓓ 2-state struct `'{…}` · ⓔ fill in `'{…}` · ⓕ string / real through `import p::*`
  * ⓖ struct-typed header override · ⓗ `p::v.a` E2002 · ⓙ `gather_local_decl_names` omissions · ⓚ generate-block `import` · ⓛ anonymous struct in `union packed`
  * ⓠ `F[i*4+3:i*4]` silent (illegal) · ⓡ block-local over a wildcard package variable read after its block · ⓣ duplicate named override accepted
  * ⓤ x/z write to a 2-state member of a 4-state struct keeps x/z (`StructFieldLayout.5`), and a READ of an enum member of a 2-state base keeps x/z, since the enum's kind is recorded `Logic` (`typedefs.rs:357`): W/row41/r1c/filed/i01_enumbit_ctl.sv `enum bit [1:0] k` never written, vita and iverilog `A ctl=xx ctlt=x`, verilator `A ctl=00 ctlt=0` (IEEE 1800 §7.2.1: converted to 2-state, `00`)
  * witness: b3_enum2_twin_pre.sv: vita `A y2=xx y3=xx y4=xx hit=0000`, verilator `A y2=00 y3=00 y4=00 hit=1111`
  * §5.2 row 41 refuses the shape in its element, and a holder's own `bit [1:0]` member beside an element `e_t [2:0] m` reads the same (W/row41/lens3-diff/d6/hh_bit_member, `s = 'x`: vita = iverilog = sv2v `A s_pre=xx s_a1=xxxx s_post=x`, verilator `A s_pre=00 s_a1=0000 s_post=0`; the flat twin hh_bit_member_ctl PRE = POST) · ⓦ package function over a non-i64 constant; `apply_import_const_funcs` · ⓧ late CU import applies · ⓨ generate bound reported once
  * ⓩ negative axis (zero demand): `A[0:-2]` (`const_bound_u32`), >64-bit negative-LSB base, `[m:l]` of a negative-LSB net (`packed.rs`) · (aa) untyped `C + D` i64 split; `$clog2(C+D)`; `byte` bound operand
