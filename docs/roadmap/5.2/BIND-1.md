(§5.2 BIND-1 · rank — F · source W/program/reorder2-2026-11-09/PLAN.txt G1 BIND, G2 BIND-1) (one bound-constant record per name, in place of the parameter maps):
* slot: R4.1 · L1, after VAL-3 (owner decision O3, 2026-11-09: the new record approved)
* scope: structure only: every output equals PRE (values, diagnostics, exit code, `.vu` and `.velab` bytes, termination, cost), PRE's differences kept as named lanes (owner, 2026-11-09)
* scope: BIND, what a name's constant value and declared type are, per scope
* evidence: parallel maps on `Elaborator` (`lib.rs`): `params` i64 (`:802`), `param_meta` (w, signed) (`:844`), `param_range` (`:852`) and its persistent twin, `wide_param_bits` `ir::ConstVal` (`:1172`), `pkg_consts` / `pkg_const_meta` / `pkg_const_range`, `array_const_vals` / `pkg_array_const_vals`, `real_param_val` (`:871`), `str_param_raw` (`:864`), `param_type_guessed`
* evidence: an absent entry is a meaning (a value-inferred width, a zero LSB, "> 64-bit stays loud"); 77 writer sites in 12 files
* evidence: readers `lookup_scoped` 21, `walk_scopes` 15, `wide_name_bits` 15, `param_sel_range` 7, `param_leaf_bits` 5, `narrow_param_bits` 2 (65 in 25 files); save / restore 6 (`instance_array.rs`)
* fix: one record type `BoundConst {value, decl: DataType + RangeFacts, provenance}` per FQ name in place of the parallel maps, one writer funnel (ER §5.1 parallel tables), accessors answering each PRE map read (65 + `param_meta`'s 39 refs)
* note: lanes: each map's absent-entry meaning (a value-inferred width; a zero LSB; a > 64-bit value never in `params`, a width from it staying loud; `param_type_guessed`)
* note: order: written where and when the maps were (the binder's fold order kept); `instance_array.rs`'s 6 save / restore sites keep their scope; `walk_scopes` visibility identical
* pins: a raw-writer grep finds only the funnel (ER §5.1); ratchet d falls (the baseline lowered)
* note: separate from VAL: a table written by producers, not a fold over expressions; two stores (scope tables here, call frames in BIND-2) because their lifetimes differ
* note: the lossy-producer class (S1v B-1's i64 bound, row 24 F1, row 29 r2, Q-MEMBSRC's park, Q-OVRXPLANE, 76) is these maps' i64 values without type or x plane, and no existing row built it (`param_meta` in docs/roadmap: Q-BITSTYPE and §2 new-U only)
* note: risk high (scope and lifetime: lesson name-keyed-rewrite-three-lifetimes); size 12 + 25 files and `lib.rs` fields; the full reader census past the named accessors (`param_meta` 39 refs, `wide_param_bits` 29, the real / str maps) is its grounding (U2)
* note: D12 never together with Q-SIDETABLES (`elaborate/src/lib.rs`'s `Elaborator` fields): VAL × BIND 22 shared files, TYPE × BIND 13
* note: it frees S1v-V, Q-OVRXPLANE, §2 gen-scope-alias-width (its table half) and row 24's F4
* note: the home rows' trip wires and their honest cost are written once, in §5.2 SYS-1
* note: from VAL-3 (§4.5.639): `check_param_decl_range` folds and checks its bounds through `range_bounds_checked`; the typed bound R6.1 builds reads a name leaf's (width, sign) from the binder, wrong today for a dropped `unsigned` (p5_uns_kw_s PRE `A bw=1 u=-8 gt=0`, iverilog `A bw=2 u=8 gt=1`), so BIND-1 and S1v-V land before R6.1 reads it
* source: W/program/reorder2-2026-11-09/PLAN.txt G1 BIND, G2 BIND-1, G7 O3
