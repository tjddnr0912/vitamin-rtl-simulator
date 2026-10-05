# DELTA — s582 round 3 (design-wide decline replaces the scope check)

POST2 = S/post2/vita md5 f491ab7b4921b912baa27b9828641232. POST3 = S/post3/vita md5 9b0c1623e3acb71130efe3acb7713a89, size 7321504.
PRE = S/pre/vita md5 4ace7617440041d0d16aee5309fded10. S/post3/slice.patch = full diff vs main (excl. docs/ENGINEERING_RULES.md, 22 file diffs);
S/post3/delta.patch = vs the POST2 tree (16 files).

## Change

- `name_inside_binds` and its call removed. The parser rule is kept: the `case … inside` branch is taken only for `case`, when the next token is not `:` or `,`.
- hdl-parser records, per parsed token vector, the first identifier token spelled `inside` that is a NAME: all `Word(Ident)` tokens with text `inside`, counted over the WHOLE
  token vector, minus the indices the two contextual-keyword sites took (`parse_inside` at expr.rs:191 and `parse_case_inside`); an `EscapedIdent` `\inside` always counts.
  It pushes `TopItem::InsideNameUse(Span)` (an appended variant: SchemaHash re-pin only, `format_version` 34 unchanged) at most once per unit.
- `Elaborator::run` (the one function every elaborate entry reaches) takes the first such record; `lower_case_inside` then refuses every statement, before lowering anything:
  "`case … inside` is not supported in a design that also uses `inside` as a name (first at <file:line>); IEEE 1800-2017 reserves `inside`".
- Escaped identifiers: the lexer keeps `\inside` as `TokenKind::EscapedIdent`, and `at_ident_kw` requires `Word(Ident)`, so neither keyword site takes it. `case (x) \inside [2:1]:` was
  already a plain case on POST2 (no misparse); pinned (ESC1 runs, = PRE = sv2v = verilator; ESC2 is declined).

## Byte-identity

195 cells (the 169 round-2 cells + t/f2a, f2b, esc1, esc2, ctl + the round-2 lens designs in S/lens_diff/r2/d and S/lens_sound/r2) + the two-file xa+xb run, on POST2 and POST3
(S/post3/cmp): POST2-accepted 98, byte-identical on POST3 91. The 7 that changed all use `inside` as a name: lens r2d_up, r2h_loopvar, sound r2f_upward, r2h_upward_gen, t/esc2, f2a, f2b.
The control t/ctl (case-inside beside the `inside` operator, no name `inside`) is byte-identical to POST2 (= sv2v = verilator). 28 cells changed in all; every one uses `inside` as a name.

## Every cell whose POST3 output differs from POST2

| cell | PRE | POST2 | POST3 | iverilog (lens: default gen / -g2005; t/f1*: -g2005) | sv2v → iverilog | verilator 5.052 |
|---|---|---|---|---|---|---|
| grounding/b2/m9_inside_ident.sv | m=0 | E3009×1 (rc=1) | E3009 design-wide (first at m9_inside_ident.sv:2) ×1 | ERR: m9_inside_ident.sv:2: syntax error; ERR: m9_inside_ident.sv:1: error: Syntax error in… (rc=7) | — | ERR: %Error: m9_inside_ident.sv:2:15: syntax error, unexpected inside; ERR: %Error: m9_ins… (rc=9) |
| lens_diff/d/d01_ident.v | A m1=1 B m2=1 C m3=1 | E3009×3 (rc=1) | E3009 design-wide (first at d01_ident.v:4) ×3 | A m1=1 B m2=1 C m3=1 | — | A m1=1 B m2=1 C m3=1 |
| lens_diff/d/d03_bk.sv | A m1=1 B m2=1 C m3=1 | E3009×3 (rc=1) | E3009 design-wide (first at d03_bk.sv:5) ×3 | A m1=1 B m2=1 C m3=1 | A m1=1 B m2=1 C m3=1 | A m1=1 B m2=1 C m3=1 |
| lens_diff/d/d21_ident2.v | G m2=1; F m1=1 | E3009×1 (rc=1) | E3009 design-wide (first at d21_ident2.v:4) ×1 | G m2=1; F m1=1 | — | G m2=1; F m1=1 |
| lens_diff/r2/d/r2a.v | W2003×1 E3010×1 (rc=1) | E3009×15 W2003×1 (rc=1) | E3009 design-wide (first at r2a.v:4) ×15 | ERR: r2a.v:110: error: Unable to bind wirememory `inside' in `top.u14'; /private/tmp/claud… (rc=1) | — | ff=1 fl=1 fa=1 fn=1 tf=1 tl=1 p=1 lp=1 sp=1 gv=1 gb=1 nb=1 im=1 ubd=1 port=1 |
| lens_diff/r2/d/r2a_c.v | W3057×1 W2003×1 E3010×2 E3009×1 (rc=1) | W3057×1 E3009×15 W2003×1 (rc=1) | E3009 design-wide (first at r2a_c.v:4) ×16 | — | — | — |
| lens_diff/r2/d/r2b2.sv | W3057×1 E3010×1 E3009×1 (rc=1) | E3009×12 W3057×1 (rc=1) | E3009 design-wide (first at r2b2.sv:3) ×15 | — | — | — |
| lens_diff/r2/d/r2d_up.v | E3010×1 (rc=1) | up m=0 | E3009 design-wide (first at r2d_up.v:7) ×1 | up m=1 | — | ERR: %Error: r2d_up.v:4:36: Can't find definition of task/function: 'inside (rc=9) |
| lens_diff/r2/d/r2f_unitfn.sv | unitfn m=1 | E3009×1 (rc=1) | E3009 design-wide (first at r2f_unitfn.sv:2) ×1 | — | — | — |
| lens_diff/r2/d/r2g.v | gfun=1 afun=1 cfun=1 | E3009×3 (rc=1) | E3009 design-wide (first at r2g.v:5) ×3 | gfun=1 afun=1 cfun=1 | — | gfun=1 afun=1 cfun=1 |
| lens_diff/r2/d/r2h_loopvar.sv | W3057×1 / forvar m=1; foreach m=1; forvar2 m=1 | W3057×1 / forvar m=0; foreach m=0; forvar2 m=0 | E3009 design-wide (first at r2h_loopvar.sv:4) ×3 | — | — | — |
| lens_diff/r2/d/r2i_blocks.sv | W3057×1 W3056×1 / ublk m=1; fork m=1; ffor m=1 | W3057×1 E3009×2 W3056×1 (rc=1) | E3009 design-wide (first at r2i_blocks.sv:4) ×3 | — | — | — |
| lens_sound/d/d03_v2005_inside_ident.v | A x=2 vs inside[1:0] m=1; B x=6 vs inside+0 m=1 | E3009×2 (rc=1) | E3009 design-wide (first at d03_v2005_inside_ident.v:2) ×2 | A x=2 vs inside[1:0] m=1; B x=6 vs inside+0 m=1 | — | — |
| lens_sound/r2/r2a_1364.v | s_port m=1; s_genvar m=1; s_gen_net m=1; s_param_real m=1; s_param_str m=1; s_param_wide m=1; s_specparam m=1; s_func_after m=1; s_func_gen m=1; s_for… | E3009×11 (rc=1) | E3009 design-wide (first at r2a_1364.v:1) ×11 | s_port m=1; s_genvar m=1; s_gen_net m=1; s_param_real m=1; s_param_str m=1; s_param_wide m… | — | — |
| lens_sound/r2/r2b_pkg.sv | w_var m=1; e_var m=1; w_fn m=1; e_fn m=1; w_par m=1; w_enum m=1 | E3009×6 (rc=1) | E3009 design-wide (first at r2b_pkg.sv:1) ×6 | — | — | — |
| lens_sound/r2/r2c_cls_let.sv | l_let m=1; c_prop m=1 | E3009×2 (rc=1) | E3009 design-wide (first at r2c_cls_let.sv:2) ×2 | — | — | — |
| lens_sound/r2/r2e_order.v | E3010×2 (rc=1) | E3009×4 (rc=1) | E3009 design-wide (first at r2e_order.v:4) ×4 | ERR: r2e_order.v:3: error: Unable to bind wirememory `inside['sd1:'sd0]' in; /private/tmp/… (rc=2) | — | — |
| lens_sound/r2/r2f_upward.v | E3010×1 (rc=1) | child upward m=0 | E3009 design-wide (first at r2f_upward.v:6) ×1 | child upward m=1 | — | — |
| lens_sound/r2/r2g_sv.sv | ifc m=1; m_imp_after m=1; pk m=1; inh_prop m=1; inh_meth m=1 | E3009×6 (rc=1) | E3009 design-wide (first at r2g_sv.sv:1) ×5 | — | — | — |
| lens_sound/r2/r2h_upward_gen.v | E3010×1 (rc=1) | child2 upward-gen m=0 | E3009 design-wide (first at r2h_upward_gen.v:7) ×1 | — | — | — |
| lens_sound/r2/r2i_implicit.v | W2003×3 / m_assign_before m=1; m_assign_after m=1; m_port_before m=1 | W2003×3 E3009×3 (rc=1) | E3009 design-wide (first at r2i_implicit.v:4) ×3 | — | — | — |
| tests/sv/esc2.sv | E3010×1 E3009×1 (rc=1) | esc m1=1 m2=1 | E3009 design-wide (first at esc2.sv:4) ×1 | — | esc m1=1 m2=1 | esc m1=1 m2=1 |
| tests/sv/f1a.sv | A m1=1 B m2=1 C m3=1 | E3009×3 (rc=1) | E3009 design-wide (first at f1a.sv:4) ×3 | A m1=1 B m2=1 C m3=1 | ERR: f1a.sv:4:26: Parse error: unexpected token 'inside' (KW_inside) (rc=1) | ERR: %Error: f1a.sv:4:26: syntax error, unexpected inside; ERR: %Error: f1a.sv:6:5: syntax… (rc=9) |
| tests/sv/f1d.sv | G m2=1 | E3009×1 (rc=1) | E3009 design-wide (first at f1d.sv:5) ×1 | G m2=1 | ERR: f1d.sv:5:18: Parse error: unexpected token 'inside' (KW_inside) (rc=1) | ERR: %Error: f1d.sv:5:18: syntax error, unexpected inside; ERR: %Error: f1d.sv:5:41: synta… (rc=9) |
| tests/sv/f1e.sv | A m1=1 | E3009×1 (rc=1) | E3009 design-wide (first at f1e.sv:5) ×1 | A m1=1 | A m1=1 | A m1=1 |
| tests/sv/f2a.sv | E3010×1 (rc=1) | up m=0 | E3009 design-wide (first at f2a.sv:8) ×1 | — | ERR: f2a.sv:8:18: Parse error: unexpected token 'inside' (KW_inside) (rc=1) | ERR: %Error: f2a.sv:8:18: syntax error, unexpected inside; ERR: %Error: f2a.sv:8:41: synta… (rc=9) |
| tests/sv/f2b.sv | forvar m=1 | forvar m=0 | E3009 design-wide (first at f2b.sv:7) ×1 | — | ERR: f2b.sv:7:14: Parse error: unexpected token 'inside' (KW_inside) (rc=1) | ERR: %Error: f2b.sv:7:14: syntax error, unexpected inside (rc=9) |
| tests/sv/xa.sv + xb.sv | E3009×1 (rc=1) | E3009×1 (rc=1) | E3009 design-wide (first at xa.sv:2) ×1 | — | ERR: xa.sv:2:17: Parse error: unexpected token 'inside' (KW_inside) (rc=1) | ERR: %Error: xa.sv:2:17: syntax error, unexpected inside, expecting IDENTIF (rc=9) |

Notes: "E3009 design-wide (first at F:L) ×n" = n statements refused, the message naming the first name use. POST2 "E3009" cells were the round-2 scope refusal ("a name
`inside` is declared here"). POST2 printed values without an error on r2d_up / r2f_upward / f2a (`up m=0`; iverilog -g2005 `up m=1`, PRE E3010), r2h_loopvar / f2b (`forvar m=0`;
PRE `m=1`) and r2h_upward_gen — silent cases, now loud. t/esc2 is SV-legal (sv2v → iverilog and verilator `esc m1=1 m2=1`, which POST2 printed): a loud over-refusal now, as on PRE
(E3010). Loud residue unchanged from POST2: t/f1f (`inside == 4'd5`), lens r2c_hier (`inside.v`), r2k_tdcast (`inside'(…)`) — E2002.

## New and changed pins (crates/cli/tests/case_inside_refused.rs)

- `f1_name_inside_in_the_design_declines_every_case_inside` (replaces round 2's scope pin): F1A 3 refusals (anchor t.sv:7:15, first at t.sv:4), F1D, F1E, F2A (upward function), F2B (loop variable).
- `escaped_inside_is_a_name`: ESC1 runs `esc m1=1`; ESC2 declined (first at t.sv:4).
- `name_use_in_another_file_declines_one_shot_and_staged`: a.sv (`package p; parameter int inside = 5;`) + b.sv (`import p::*; case (x) inside [4'd4:4'd6]:`) refused one-shot and through
  `vita vcmp` → `vita velab` (first at a.sv:2); control CTL runs, = sv2v = verilator.
- unchanged: `f1_inside_as_a_label_name_still_runs` (F1B, F1C), `f1_binary_operator_after_the_word_is_a_parse_error` (F1F), `p01_casez_casex_inside_stays_a_parse_error`.

## Gates (round 3)

cargo nextest run -p hdl-ast --locked → 5 passed (hash re-pinned); -p hdl-parser → 79 passed; -p cli --test case_inside --test case_inside_refused → 47 passed;
-p cli (whole) → 7966 passed, 1 skipped, 450 s; clippy --workspace --all-targets -D warnings rc 0; fmt --check rc 0.
Backends interp/vm/native + staged vcmp→velab→vrun on the running cells t/ctl, esc1, f1b, f1c: byte-identical to default. Staged refusals at velab: f1a 3, f1d 1, f1e 1, f2a 1, f2b 1, esc2 1,
xa+xb 1 (one .vu). Library compose (`vcmp --work` per file, `velab -L … --top t`): a child module declaring `reg [3:0] inside` in one unit and the case inside in another → refused
("first at byte 25": the -L path has no source-map resolver); the same with the child renamed runs `m=1`. A package in another unit cannot be measured on -L (pre-existing:
"import from unknown package `p`", the compose loads the instantiation closure only).
