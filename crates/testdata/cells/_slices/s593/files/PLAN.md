# §4.5.593 start decision + implementation plan — §2 🆕 W + 🆕 S (a) i64 half
status: DONE — decision NO-GO (BLOCKED on a new prerequisite row); retry plan below
PRE = $S/s592/pre/vita md5 86a2d84a21d0329c57541e51ae9eb4d2 (main 75f46453, release)
CUT = $S/s593/post5/vita md5 ebfa0e02afb30e373f6804b242ee8aad (debug; patch g/cut_post5.patch); post1 = W alone (ffe70ae3…)
audit dir = $S/s593/a (cells p3 p5–p12, *.summ; classifier cls3.py; 666 harness h666.py / h666b.py / perline.py)
ref = iverilog = sv2v, else ≥2 agreeing oracles; "1or" = one running oracle

## Intent
Land the sign of a constant typed by an overridden type parameter (W) together with the i64 constant `==?` / `inside` at the
common width (S (a) i64 half), since each alone descends — without moving any PRE-correct cell (ER §2.1, §2.2).

## Decision: NO-GO now. W routes T-typed constants onto a pre-existing walk defect it cannot exclude; file it first.
Blocker (new, measured, not in the grounding): a T-typed constant made signed by W, folded in a constant region whose width
`const_self_width` cannot give (a parameter-count replication `{N{…}}`, an unpacked-array-parameter element), goes
correct→silent-wrong even with no wildcard. The explicit signed twin is already wrong on PRE, so it is the walk's defect
(ER §2.5: "a wrong twin is the walk's defect: exclude the shape, file the walk as prerequisite, never patch it in the routing
slice"; ER §2.2: "close every leaking sink before changing the value that leaks"). W changes a declaration-level meta read by
every consumer, so no opt-in excludes the shape; S (a) without W keeps PT5 loud→wrong (`R=0`, oracles 1). No narrower cut ships.
- p12/Tv12 (T-typed, `.T(logic signed [7:0])`, `localparam T X = -4; localparam int N = 2;`): PRE `L=252 M=1 K=1 vb=5`,
  CUT `L=4294967292 M=0 K=0 vb=4`, iverilog / sv2v / verilator `L=252 M=1 K=1 vb=5`
  (L = `X + {N{1'b0}}`, M = `(X + {N{1'b0}}) == 8'hFC`, K = `… > 8'd100`, vb = `$bits(logic [M+3:0])`).
- p12/Ev12 explicit twin `logic signed [7:0] X = -4`: PRE = CUT `L=4294967292 M=0 K=0 vb=4` (3 oracles 252/1/1/5);
  Ev12c literal count `{2{1'b0}}`: PRE = CUT = 3 oracles `L=252 M=1 K=1 vb=5` (root = the count is unsized).
- p11 (T-typed, plain compare / arithmetic, cut = post1): 17 OK→WRONG (Tu7–Tu12 × lp gi gc rb: `L=1`→`0`, `GI=then`→`else`,
  `GC=item`→`def`, `vb=5`→`4`; sv2v + verilator = PRE; iverilog refuses the unpacked array decl); 25 controls OK=.
- p8 (T-typed under `==?`, cut = post1): 8 OK→LOUD (Wu1 `(C ? X : A[1])`, Wu2 `(X | A[1])`, Wu4 `(X + {N{1'b0}})` × lp gi rb:
  E3009 / E3010 / "a reference to net/variable `A`"), 1 OK→WRONG (Wu4_rb `vb=5`→`vb=1`), gc ×3 `item`→`def` (sv2v = PRE).
- p10 explicit twins on PRE: Eu1/Eu2/Eu4 E3009 / E3010 / `vb=1` / `GC=def`; Eu7–Eu9 `L=0 GI=else GC=def vb=4`
  (sv2v, verilator `1 then item 5`).

## Audit
1. 666-variant harness (cmp4.py set; PRE vs CUT, oracle-run every mover; per output label):
   - 666 variants / 343 files; 126 move. LOUD→OK 367 (ref) + 169 (sv2v-only) + 20 (verilator-only); WRONG→OK 39 + 5 + 1;
     OK→anything 0.
   - LOUD→WRONG 5 labels, each a pre-existing line unmasked by the cut fixing sibling lines: Q3 -DPSEL(-DIV) `PSEL 1`
     (iverilog + sv2v `x`), -DFNRX `FNRX 1`, -DTDX `TDX 1` (sv2v `x`, iverilog `1`); isolated (a/p5/iso_*) PRE = CUT
     (`PSEL 1`, `FNRX 1`, `TDX 1`, `REPX 0`) = PROBE_CATALOG §4.5.580/581 x-valued `==?` sinks. Q9 -DIV `UEV 0`
     (iverilog/sv2v 16, verilator 0) = §2 🆕 L untyped-width split.
   - LOUD→noref 273: 258 P06/P06v labels all oracles refuse (their -DNOGC twin = verilator on all 20 labels); 11 CUT = sv2v vs
     verilator; 2 = iverilog; 1 = verilator; P2c `OVX v inside {P} = 0` (sv2v `x`, verilator `1`) = row 15 + 🆕 S (b).
   - WRONG→WRONG 2 composite lines (H03 `bits 1 1 1`→`2 2 1`, K02): every sub-value moved toward the oracles or stayed.
   - LOUD→LOUD (28 variants): every CUT error line is byte-identical to one of PRE's (CUT ⊂ PRE; the dropped lines fold).
2. 160 loud→value by consumer → AC sinks / AE (p3: S1–S8 shapes × lp gi gc rb rep psw ad; p7: B02 enum, DP3 defparam,
   B07 package, B10 generate, PT5 T-typed, TRN ternary, ADD, SHR × the same 7):
   - grounding's 160 by consumer: lens c15 lp 36, gi 36, enum/fn-arg/gen-for/indexed base+width/part-select/`$clog2`/ternary
     18, PT 5, package 2, generate 1, defparam 2; c8 28; c9 12; c11 3; c12 2; c5 1.
   - non-AE families in every sink: 0 regressions (p7 21 LOUD→OK, 28 WRONG→OK, 7 OK=; p3 S1–S8 LOUD/WRONG→OK; PRE's
     sink fallbacks `vb=1`, `r=00000000`, `ps=00000001`, `GC=def` become the oracles' `vb=5`, `r=10101010`, `ps=00000101`, `item`).
   - 🆕 AE, a call in the left operand (the run happens inside the newly opened lane — §5.2's rule): A1/A2/A3/A6, xADD, xTRN,
     xSYS (`$clog2(fx(2)+4'd1) inside {…}`): LOUD→WRONG 10 (e.g. A2_lp PRE E3009, CUT `L=1`, 3 oracles `L=x`; A2_gi CUT
     `then`, iverilog/sv2v `else`), OK→WRONG 4 (A2_gc, A3_gc, xADD_gc, xTRN_gc: PRE `GC=def` = iverilog + sv2v, CUT `item`).
     xCAT (call inside a concatenation) does not move (the concat part is not interpreted).
   - 🆕 AE parameter channel (no run in the lane; the parameter is already wrong on PRE): A5 (`localparam PA = fxs(2)`), xDP
     (`defparam u.P = fxs64(2)`), xPT (`.PV(fxs64(2))`): LOUD→WRONG 6, OK→WRONG 3 (gc right by accident on PRE); control twins
     a/p9: PRE `PA=-4`, `P=-4`, `PV=18446744073709551612` (CUT `-4`), 3 oracles `x`.
3. Fallback soundness:
   - `const_compare_special` has one caller (const_fn_width.rs:552); the cut calls `const_wildcard_i64` right after it in the
     same `env.is_empty() && envw.is_empty()` block, so every node that reached PRE's masked arm reaches
     `const_wildcard_masked_pre` on any `_inner` decline (op, pattern not an x/z literal, fill, parse, no x/z bit, unknown width,
     w > 64, eval None). The string arm still runs first.
   - `masked_pre` ≡ PRE's arm: same predicates (WildEq/WildNe, unparenthesised Sized literal with an x/z bit, a ≥ 0,
     single word, bit 63 clear); only the order of the pure `const_int_selfdet(lhs)` differs. The fold helpers emit no
     diagnostics (const_fn*.rs, const_wide.rs, const_str.rs: 0 diag calls); a decline's E3009/E3010 text is built by the
     consumer from the AST, so it is PRE's (666: on all 28 LOUD→LOUD variants CUT's error lines ⊂ PRE's, byte-identical).
   - Side effect: on a decline after `eval_const_env_at`, the lhs is walked twice before `selfdet_bits_i64` (3 walks vs PRE's 2):
     3^d vs 2^d on a chain of declining wildcard compares; a call-bearing lhs would run the interpreter twice (removed by the
     retry's call exclusion). Unmeasured (debug CUT): measure in release.
   - i64 answers ≠ PRE where PRE = oracle: only the AE cells above (gc). None elsewhere (666, p3, p7).
   - Array container (cut's extra: `parse_array_param` carries `shape_param`): W3 `A[0] ==? …`, W6 `(C ? X : A[1]) ==? …`
     OK→LOUD 10, gc `item`→`def` 2 (sv2v = PRE); explicit signed array twin a/p6 arrE PRE = CUT E3009, gc `def`, bound refused
     (verilator `L=1 L2=1 vb=5`; sv2v drops array sign; iverilog refuses) — the same walk (no unpacked-element self width).
4. ptS_ctl row: CUT does not touch it (PRE = CUT on a/p4). Different root, not T-specific: `localparam logic [W-1:0] X = -4`
   under an overridable header `W` (ptW_ctl) `sb=5 X=252`, per instance ptWo `t.u16 sb=5`, `t.u8 sb=5`; 3 oracles `sb=261`,
   `t.u16 sb=65541`; literal `localparam int W` twin ptL_ctl right. Parser `params.rs` `param_item_to_module_item` records the
   raw initializer when `const_param_fits` cannot fold the range (`_ => true`). Out of W's fix path → PROBE_CATALOG row.

## Rows to file (docs commit; no product change)
- NEW §2 row X (start-order, prerequisite of 🆕 W): "a signed constant in a constant region whose width the walk cannot give
  folds sign-extended into an unsigned region: `logic signed [7:0] X = -4; int N = 2;` `X + {N{1'b0}}` `L=4294967292`,
  `== 8'hFC` 0, `> 8'd100` 0, a bound `vb=4` (3 oracles 252 / 1 / 1 / 5; literal count `{2{1'b0}}` right); beside an unpacked
  array parameter element `(X | A[1]) == 8'hFE` / `(C ? X : A[1]) == 8'hFC` `L=0 GI=else GC=def vb=4` (sv2v, verilator
  1 / then / item / 5); under `==?` E3009 / E3010 / `vb=1` / gc `default`; a signed array element `A[0] ==? …` E3009, gc
  `default` (verilator `L=1`); `const_fn_width.rs` `const_self_width`: `K::Replicate` folds its count with the literal-only
  `const_eval_u32`, no unpacked-element arm; an unknown width turns masking off in `eval_const_env_at`'s leaf arm; holds 🆕 W
  (Tv12, p11 17, p8 12); fix: make the width knowable (count by the scope fold, element by its declared range; ER §2.5),
  census every `None` arm of `const_self_width` / `const_select_self_width` with a signed sibling; 3 oracles; OPEN".
- §2 🆕 W: "… BLOCKED (row X); one slice with 🆕 S (a)'s i64 half (each alone descends, §4.5.593); its array container
  (`parse_array_param`) also on row X's unpacked-element arm (W3/W6)".
- §2 🆕 S retry line: "with 🆕 W after row X: `eb9d3b69`'s i64 half, every decline → PRE's compare, a left operand holding a
  call kept on PRE's compare (🆕 AE: A1–A3, xADD, xTRN, xSYS); §4.5.593 ran §4.5.580's 666 variants on that cut (0 correct→
  other per label); the parameter channel (A5, xDP, xPT) is 🆕 AE's; wide half (LP, st1, `||` S9) after 🆕 T, U".
- §5.2: row X before W (new row 8); row 9 "🆕 W + 🆕 S (a)'s i64 half"; row 10 "🆕 T, then 🆕 S (a)'s wide half". "Do not start"
  W line: blocked on row X. LOOPROMPT NEXT #4/#5 follow. REMAINING_WORK:18 (W ↔ S (a) mutual, both behind row X).
- PROBE_CATALOG: §4.5.593 grounding — the parse-time table row (a/p4 values above), code site hdl-parser `params.rs`.
- Optional held cells (tests-only): Tv12 PRE-RIGHT guard and Ev12 KNOWN-WRONG witness in
  `generate_case_and_wildcard_prerequisites.rs` (§4.5.592 edits the same file: rebase).

## Retry plan (after row X lands; re-freeze PRE = that main; re-run every set above on the new PRE)
Lane table (ER §10.2): parser typedef arm → `ParamPrefix.shape_param` (measured c2 c3 c6); scalar `ParamDecl.shape_param`
(measured; array arm opted out unless row X sized unpacked elements — then re-measure W3/W6/arrL/arrE); elaborate
`param_decl_width_opt` range arm (measured H/L/B/G/ifc/defparam/positional); param_meta readers (measured c2 c3 c6 c10, p8 p11
must be OK after row X); `const_wildcard_i64` + fallback + call exclusion (measured 666, c4 c8 c9 c11 c12 c15, matrix 174,
census 252, p3 p7); parser `const_locals` opted out (PROBE_CATALOG); hdl-ast schema hash re-pin.
Steps (each verified):
1. hdl-ast `ParamDecl.shape_param: Option<Ident>` before `span` with a doc comment like `NetVarDecl`'s; parser
   `ParamPrefix.shape_param` (typedef arm, no signing keyword) → scalar `ParamDecl` only; `None` at module_items.rs:795,
   type_params.rs ×3, type_param_packed.rs:65, elaborate tests/mod.rs. Verify: `cargo check --workspace --all-targets --locked`.
2. Schema hash (CONTRIBUTING › Frozen types): sim-ir does not depend on hdl-ast (grep: 0) → no `format_version` bump, no
   sim-ir golden; run `cargo test -p hdl-ast --test schema_hash --locked`, paste the printed hash into `EXPECTED`, prepend a
   dated "Re-pinned … §2 🆕 W `ParamDecl.shape_param` … All `.vu` stale; no sim-ir change, `format_version` unchanged" entry.
   Verify: test passes; `header.rs` `CURRENT_FORMAT_VERSION` untouched; a PRE-written `.vu` exits 2 (E9002) under POST.
3. elaborate params.rs:306 `self.shape_signed(p.signed, &p.shape_param)`. Verify: c2 36 silent→ok, controls byte-identical.
4. S (a) i64 half from the cut (`const_wildcard_i64` / `_inner` / `const_wildcard_masked_pre` / `wildcard_match`, the masked arm
   leaving `const_compare_special`, the call at const_fn_width.rs:552) plus, at the top of `_inner` after the op/pattern checks,
   `if crate::param_query::ast_any(lhs, &|x| matches!(x.kind, ast::ExprKind::Call { .. })) { return None; }` (→ PRE's compare;
   ER §2.5 over-seed stays loud); one `or_else` fallback (drop the duplicate w > 64 return or keep it with one comment); fix
   the stale `masked_pre` doc ("every decline", not "past 64 bits"). Verify: A1–A3/A6/xADD/xTRN/xSYS = PRE byte-identical.
5. Pins: convert (LOOPROMPT §5, oracle text moved with each): P3 ×2 → `lt0=1 bits=64`, `TP=-1 lt0=1 bits=4`; PT5 REFUSED →
   `R=1 lt0=1`; X13 KNOWN-WRONG → `X13 item` (sv2v; iverilog refuses `inside`, X11 twin `item`; verilator refuses); every
   failing constant-context pin in `inside_wildcard.rs` and `sim-engine/tests/const_domain_semantics.rs` (eb9d3b69 touched both);
   find them by running `cargo nextest run -p cli --locked --no-fail-fast` and the sim-engine tests on POST. New file
   `type_param_typed_constant_sign.rs`: oracle-pinned W lanes × consumers, override axes, sign drop, fill, controls, Tv12;
   S (a): the four fixed cells, MC4, AC-sink cells (p3 S1–S8, p7 families), guards r3a / P68 / arrE (PRE), AE guards (PRE
   output + oracle text, REFUSED / KNOWN-WRONG), the parameter-channel cells with their oracle `x` (KNOWN-WRONG, 🆕 AE).
6. Mutants (write expectations first; battery `cargo nextest run --workspace --locked --no-fail-fast`): M1 :306 → `p.signed`
   (P3, L_d_s8); M2 parser shape_param → None (P3); M3 drop `const_wildcard_i64` (MC4, SP, `(4'd15+4'd1) ==? 5'b1?000`);
   M4 drop `or_else` (P68 if the w>64 return goes too; else find a width-unknown killer that survives row X); M5 drop the call
   exclusion (A2_gc, xADD_lp); M6 `sg` ignores `cv.signed` (S1_lp); M7 drop `.max(pw)` (`(4'd15+4'd1) ==? 5'b1?000`); M8 drop
   `InsideEq` (`4'b1100 inside {4'b1?00}`); M9 unsized flag false in `pattern_ext_fill` (S5 / `'b?100` over 40 bits).
7. Byte-identity: no T-typed decl → `shape_param` None → `shape_signed(s,&None)=s`; T-typed without a sign-changing override →
   `T$s` bit 0 = default = `p.signed`; no constant `==?`/`!=?`/x-z `inside` → route not entered; every decline and every
   call-bearing lhs → PRE's arm verbatim. Verify: release POST `.velab` byte-identical on the 11 corpus + 4 examples (only the
   `.vu` upstream digest moves), `corpus-runner run` 11/11 no page change, staged chain (`--features separate-bins`).
8. Gate (CONTRIBUTING): nextest + doctests workspace, clippy `--workspace --all-targets -D warnings`, fmt, product shape.
   Perf probe: a 16–24 deep declining `==?` chain, release PRE vs POST, both orders.
Risks: §4.5.592 (V, gen_decision.rs, same held-cells file) landing first → re-measure X13/X23 and c5 pins; the parameter
channel (A5, xDP, xPT; gc right-by-accident→wrong) needs the lens/moderator's explicit ruling (§5.2's rule names runs, ER §2.2
"never widen a loud into a silent"); row X may change which declines remain (re-check M4's killer).

## What remains after the retry
🆕 S (a) wide half (LP `{64'd0, (… ==? …)}`, st1, 65-bit binders, generate-for condition, two compares under `||` — p3 S9
`I inside {4'b1?00, 4'b0011}` stays E3009/E3010 and sink `vb=1` on CUT) after 🆕 T and 🆕 U-b (BLOCKED on 🆕 AE);
🆕 T (d4dc9c26's two-way label compare) after 🆕 V (§4.5.592) and U-b; 🆕 S (b) unchanged; W's array container if row X does
not size unpacked elements (arrL/arrH/arrInt/arrW/arrDrop verilator-only silent).
