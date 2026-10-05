# §4.5.593 grounding — §2 🆕 W (type-parameter-typed constant loses override sign)
status: DONE (see end)
PRE = $S/s592/pre/vita md5 86a2d84a21d0329c57541e51ae9eb4d2 (main 75f46453)

## Q0 repro (PRE, 3 oracles) — $S/s593/g/c1
- PT5c: PRE `R=0`; iverilog `R=1`; sv2v `R=1`; verilator `R=1`
- dF3:  PRE `TP=15 lt0=0 bits=4`; iverilog/sv2v/verilator `TP=-1 lt0=1 bits=4`

## Q2 code census (in progress)
Root (read): `hdl-ast` `ParamDecl` has `signed: bool` and NO `shape_param` slot. `hdl-parser/src/params.rs`
`parse_param_prefix` typedef arm: `signed = expl0.unwrap_or(info.signed)` stamps the DEFAULT's sign of
`T` (TypeInfo from `type_params.rs:265`, range `[T$w-1:0]`, `shape_param: Some("T$s")`), and does NOT call
`note_uncarried_*`, so the `T$s` guard narrows to the arity bits (§4.5.479) and the override binds silently.
Compiler census (field renamed in $S/s593/gwt, `cargo check --workspace --all-targets --keep-going`, log g/chk1.log):
- readers, elaborate: params.rs:306 (range arm of `param_decl_width_opt` — the only one a T-typed decl reaches:
  range=Some, ty=Implicit), :313 (Integer arm, unreachable: ty=Implicit), :1951/:2145/:2182/:2497 (gated
  `range.is_none()` / untyped: unreachable), :2508 (`override_typed`, reached; explicit-twin parity),
  expr_size_hier.rs:434 (range=Some → slot unbound either way).
- readers, parser (parse-time `const_locals` table): params.rs:923/937 (`const_param_fits`), :1014–1022
  (`param_const_shape`) — per-MODULE, cannot know an instance's `T$s`.
- constructors: params.rs:650 (scalar), module_items.rs:795, type_params.rs:175/187/382, type_param_packed.rs:65,
  elaborate tests/mod.rs:395.
- sibling container: `parse_array_param` (`localparam T A[2]`) builds `NetVarDecl { shape_param: None }`.

## Q3 cells c2 (lane × consumer, override `.T(logic signed [7:0])`, value -4) — $S/s593/g/c2, c2.summ
Lanes: H header `parameter T X`, L body `localparam T X`, B header-less body `parameter T X`, G generate-block
`localparam T X`. Consumers: d `%0d`/`<0`/`$bits`, a `>>> * / %`, e extension to 16/int, c procedural case,
r derived localparams, g generate-if, k generate-case, w range `[X+8:0]`, x compare/mixed add, s `$signed/$unsigned`.
- PRE: 36 cells (4 lanes × d,a,e,c,r,g,k,w,x) silent-wrong vs 3 agreeing oracles (e.g. L_d_s8 PRE `X=252 lt0=0 b=8`,
  iverilog/sv2v/verilator `X=-4 lt0=1 b=8`; H_w_s8 PRE `vb=261`, oracles `vb=5`; H_k_s8 PRE `gcase=def`, oracles `gcase=m4`).
- 45 controls (no override, signed default `_ctl`; unsigned default `_uctl`; explicit twin `E_*`) = all 3 oracles.
- 9 `s` cells: oracle SPLIT on `%0d` of `$signed/$unsigned` (iverilog `us=-4`, verilator `us=252`), the explicit
  twin E_s_twin splits identically → not W; PRE = verilator.
- prototype post1 (one-site, see Q5): 36 silent→correct (all 3 oracles), 45 + 9 byte-identical, 0 other moves.

## Q3 cells c3 (override-type axis, fill/negative defaults, sign drop, containers) — g/c3, c3.post1.summ
PRE→post1 over 145 cells: 68 silent→correct (all 3 oracles) + 1 silent→correct (passthru: sv2v+verilator, iverilog
refuses the design); 49 correct, byte-identical; 21 loud, byte-identical; 6 wrong/split, byte-identical (below).
- override axis (header `.X(-4)`, localparam `-4`, localparam `'1`): s4 s8 int byte shortint longint integer
  `bit signed [7:0]` s40 s64 s80 s100 typedef `s8_t` package `p::ps8_t` — every one silent→correct
  (e.g. ovH_s80 PRE `X=1208925819614629174706172 lt0=0 b=80`, post1 and 3 oracles `X=-4 lt0=1 b=80 shr=-2`).
- sign DROP (signed default `logic signed [7:0]`/`int`/`byte`, unsigned override u8/`int unsigned`/`byte unsigned`):
  18 silent→correct (dropLf_int_uint PRE `X=-1 lt0=1 b=32`, oracles `X=4294967295 lt0=0 b=32`).
- `'0` fill (ovL0_*): correct on PRE and post1 (sign-free value).
- nested `T2 = T` (header and localparam type), `typedef T tt`, pass-through `.T(T)`, positional, `defparam u.X`,
  interface header and body: silent→correct.
- packed struct override (sts/stu, 12 cells): E2002 PRE=post1 (known PROBE_CATALOG §4.5.568). class `#(type T)`:
  E2002 parse PRE=post1. `const T X`: E2002 PRE=post1.
- 2-state axis: x/z-valued T-typed localparam E3009 PRE=post1 (explicit twin same); header `.X(4'bx01z)` binds `0011`
  on PRE=post1 for T-typed AND explicit twin `parameter bit [3:0] X` (iverilog `0010`) = §2 row 15 (x/z override plane),
  not W.
- array param `localparam T A[0:1] = '{-1,-2}` (arrL): PRE=post1 `A0=15 A1lt0=0`; verilator `A0=-1 A1lt0=1`; iverilog
  sorry; sv2v `A0=15` but sv2v also gives 15 on the explicit signed twin (arrL_twin, verilator/PRE `-1`) → sv2v
  disqualified; 1 oracle + hand-IEEE → W's sibling container (`parse_array_param` NetVarDecl `shape_param: None`).
- parse-time table (NO override): ptS_ctl `typedef struct packed {logic [X+8:0] a;}` over `localparam T X = -4`,
  default `logic [7:0]`: PRE=post1 `sb=5`, 3 oracles `sb=261`; ptG_ctl `g[X+5].K`: PRE=post1 `gk=1`, iverilog and
  verilator refuse (no g[257]). Root: `hdl-parser/src/params.rs` `param_item_to_module_item` records the raw
  initializer (`const_param_fits` → `_ => true` when `[T$w-1:0]` does not fold at parse time). Independent of overrides:
  a separate row (not moved by W's elaborate fix; ptS_s8/ptG_s8 right by the raw value = signed override).

## c6 (consumers outside the declaring lane) — g/c6
11 silent→correct (hier `u.X` body+header, routine body `f()`/`g()`, constant function, alias body/header,
`localparam T Y = X+1`, untyped derive, `repeat (X+6)`, width s16/s3), port width port_s8 → sv2v+verilator `ob=5`
(iverilog refuses), aliasG_s8 value/sign → oracles but `b=32` stays (§2 row "generate-scope alias binds 32 bits",
aliasG_ctl `b=32` too); 12 controls byte-identical.

## Q1 §4.5.479 / §4.5.483 pins on PRE (g/c5: the six pin files' designs dumped by an instrumented `run()`, 216 unique)
- §4.5.479 = `crates/cli/tests/type_param_shape_override.rs` (26 tests), §4.5.483 = `type_param_shape_cast_struct.rs`
  (21 tests); they assert VARIABLE / port / tf-port / cast / struct-member shapes (`T v; v = -1; m1 bits shr neg x=`),
  per-axis F4004 louds (2-state cast/struct, enum base, RETURN type, dim count, carrier names) and controls. None
  declares a constant typed by `T` (`parameter T X` / `localparam T X`), so W is outside both pins' measured set; the
  only T-typed constant pins at HEAD are §4.5.581's P3 cells (generate_case_and_wildcard_prerequisites.rs) and
  `type_parameters.rs` `parameter T X = T'(300)` (cast in the value; byte-identical).
- 76 designs from the two files on PRE: 26 = all 3 oracles; 20 loud as pinned (oracles run them; filed §3 ⑤ⓕ /
  Do-not-start rows); 30 differ only where an oracle is disqualified or refuses: verilator `x=0…0` on every 4-state
  `%b` column (test header records it), sv2v `x=x…x` on 2-state columns, iverilog `Invalid module instantiation` on
  `n #(.T(T))` pass-through, sv2v `syntax error` on multi-dim, `$typename` (iverilog/sv2v undefined, verilator
  `PARAMTYPEDTYPE 'T'`; vita `bit[7:0]`, no oracle). Every value column (bits m1 shr neg) = every running oracle.
  Verdict: both pins still right per both oracles on the columns they assert.
- prototype post1 (debug, md5 ffe70ae34ca91c690d9420e8c52ada65): 214/216 pin designs byte-identical to PRE; 2 move
  silent→correct = the two P3 KNOWN-WRONG pins (`p3_a_parameter_typed_…` `lt0=0`→`lt0=1`, `p3_a_fill_typed_…`
  `TP=15 lt0=0`→`TP=-1 lt0=1`; all 3 oracles). `cargo test` on post1: those two FAIL (they assert today's wrong value),
  every other test of the six files passes (header_import_param 47, cast_struct 21, override 26, type_parameters 7,
  typed_param_user_type 21, gcwp 10/12).

## Q6 what eb9d3b69 needs from W (binaries: post3 = HEAD + eb9d3b69's net elaborate diff vs e54fa74a, md5
18f6ab435e59fb29c92e7c20d1b1e00b; post2 = post1 + the same diff, md5 c710901bf34b80701170e6b7a6dd3cc8; both debug)
- `const_wildcard_i64` computes `sg = self.const_signed_env(lhs, envw) && cv.signed` and evaluates the left operand by
  `eval_const_env_at(lhs, …, w, sg)`; for a T-typed name both read the name's recorded meta — W's
  `param_decl_width_opt` range arm is what makes that meta carry the override's sign. Nothing else: the width
  (`const_self_width`) and the stored bits are already right on PRE (PT5b `$bits(PV)` 64 = oracles).
- PT5 (`s_a_wildcard_over_a_type_parameter_typed_constant_stays_loud`, `PV ==? 4'sb1?00`, oracles `R=1 lt0=1`): PRE
  E3009; eb9d alone `R=0 lt0=0` (round-3 F2 loud→wrong); W alone E3009; W+eb9d `R=1 lt0=1`.
- what W does NOT give it: the round-3 S3-1 blocker is independent of W — `({N2{4'b1100}} ==? 8'b1?00_1100)`
  (`s_a_replication_left_operand_keeps_its_value`, oracles `R=1`): PRE `R=1`, eb9d and W+eb9d E3009. The retry still
  needs every `const_wildcard_i64` decline to fall back to PRE's masked compare (ROADMAP 🆕 S retry line).
- §4.5.581's other held cells (A1D, d1p, d1w, T1/L06/S06, N09): PRE = W = W+eb9d (U, V, T are their prerequisites);
  X13 moves to `item` under eb9d with or without W (its `inside` label).

## BLOCKER found: W alone routes T-typed constants into the constant-wildcard decline and its sinks (g/c8 c9 c11 c12)
`const_str.rs` `const_compare_special` (HEAD's masked `==?`/`!=?` compare) answers only for a NON-NEGATIVE left value
(`a >= 0`), else `None`. PRE read a T-typed constant overridden signed as its unsigned default (252), so the masked
compare answered — right by accident wherever the compare is unsigned or same-width. W makes the value -4 → `None` →
the consumer's decline path: localparam/gen-if E3009/E3010 (correct→loud), and the 🆕 AC sinks (range bound
`const_bound.rs` catch-all, array dim, `+:` width, generate-case label = no match) silently (correct→silent-wrong).
post1 = the explicit twin on PRE in every cell (L_* on post1 ≡ E_* on PRE), and post2 (W + eb9d3b69) restores
every one (and fixes the explicit twins). vita-only so far (oracles pending in the tables below):
- correct→loud on post1: L_wn_* ×4, H_wn_s8, L_w8_* ×4, H_w8_s8, L_gu_* ×4, H_gu_s8 (c9); L_add0, L_sub0, L_wsg (c11);
  L_ksc, L_tern (c12); H/L_wu_s8, H/L_wu_int (c8).
- correct→silent-wrong on post1: L_bu_* ×4 + H_bu_s8 `vb=4`→`vb=1` (c9); L_adim `asz=4`→`32`, L_ku0 `GC=item`→`def`,
  L_pb `vb=4`→`1`, L_psel `ps=11`→`1` (c12).

## Oracle-confirmed move tables (ref = 3 oracles agreeing, or 2 agreeing with the third refusing; arrays: verilator)
W alone = post1 (carrier + params.rs:306). Cut = post5 (debug, md5 ebfa0e02afb30e373f6804b242ee8aad) = post1 +
array container (`parse_array_param` result carries `shape_param`) + eb9d3b69's `const_wildcard_i64` + every decline
of it falling back to PRE's masked compare (`const_wildcard_masked_pre`).
| set | cells | W alone (post1) vs PRE | cut (post5) vs PRE |
|---|---:|---|---|
| c2 lanes×consumers | 90 | 36 silent→ok; 45 ok=; 9 split= | same as post1 |
| c3 override/containers | 145 | 69 silent→ok (1 two-oracle); 50 ok=; 18+3 loud=; 5 wrong/split= | + arrL silent→ok (verilator) |
| c4 PT (§4.5.581) | 14 | 2 silent→ok; 4 silent→LOUD; 1 wrong=; 5 loud= | 14/14 ok (5 loud→ok, 7 silent→ok) |
| c5 pins | 216 | 2 silent→ok; 214 byte-identical | + PT5 loud→ok; X13 label `default`→`item` |
| c6 outside lane | 27 | 11 silent→ok (+port_s8 2-oracle, aliasG_s8 value) | same as post1 |
| c8 `==?`/inside/gen-if/run-time | 75 | 4 OK→LOUD; 14 silent→LOUD; 10+10 →ok | 75/75 ok |
| c9 unsigned pattern, bounds | 54 | 15 OK→LOUD; 5 OK→WRONG (bu vb 4→1) | 52/52 ok (+2 noref=) |
| c10 shifts/pow/real/rep/sel | 39 | 10 silent→ok; 24 ok=; 2 split= | same as post1 |
| c11 expression lhs | 30 | 3 OK→LOUD; 2 silent→ok; L_bs wrong→wrong(other value) | 24 ok, 3 loud everywhere= |
| c12 🆕 AC sinks | 24 | 2 OK→LOUD; 4 OK→WRONG (adim 4→32, ku0 item→def, pb 4→1, psel 11→1) | 18 ok, 6 loud everywhere= |
| c13 arrays | 9 | 0 moves | 5 silent→ok (verilator; iverilog sorry, sv2v drops the sign on every twin) |
W alone totals over c2–c12: OK→LOUD 24, OK→WRONG 9, silent→loud 18, silent→ok 152 (+10 loud→ok). Cut: 0 cells
whose PRE answer = oracle move to anything else, on every set above (lens set c14 pending, below).

## Q4 lane table (ER §10.2) and byte-identity
| shared function / table | lanes reached | status |
|---|---|---|
| parser `parse_param_prefix` typedef arm → `ParamPrefix.shape_param` (only from `TypeInfo.shape_param`, i.e. an overridable `parameter type` or an alias of one, and no signing keyword) | module header, header-less body `parameter`, body `localparam`, generate `localparam`, interface header/body, package (shape None by construction: package type params are non-overridable literals), class (E2002 before it) | measured c2 c3 c6; package opted out (None) |
| parser `finish_param_assignment` scalar arm → `ParamDecl.shape_param`; array arm → `NetVarDecl.shape_param` | as above; array params | measured c2 c3 c13 |
| elaborate `param_decl_width_opt` range arm (params.rs:306) → `shape_signed` | `bind_one_param` (header, interface, named/positional/defparam overrides), instance.rs body binder, generate.rs, package.rs (None), iface_inst | measured H/L/B/G/ifc/defparam/positional; `-G` not measured (only reaches a TOP, whose `T$s` is its default → same sign) |
| readers of `param_meta`/`param_range`/`params`/`wide_param_bits`/`hier_params` for that key | i64 fold, width-aware fold, wide (>64) domain, run-time lowering, gen-if, gen-case, ranges, ports, hier `u.X`, routine bodies, constant functions, untyped aliases | measured c2 c3 c6 c10 |
| `const_str.rs` `const_compare_special` masked `==?` + 🆕 AC sinks (`const_bound.rs` range catch-all, array dim, `+:` width, gen-case label no-match) | every constant wildcard compare over the routed name | measured c8 c9 c11 c12: W alone descends; cut answers via `const_wildcard_i64` |
| `wildcard_eq.rs` `const_wildcard_i64` + fallback (eb9d3b69 re-derived) | comparison arm of `eval_const_env_at` (all constant lanes incl. generate-case labels) | measured c4 c8 c9 c11 c12 c14; UNMEASURED on the cut: §4.5.581's 174-cell consumer matrix, 666-variant harness, 252-cell generate-case census, round-3 F1 variants, velab/corpus identity |
| parser `const_locals` (`param_item_to_module_item`, reads `p.signed` + raw value per module) | struct member layout, generate-hier index, `[C+D:0]` parse folds | opted out (not edited; ptS/ptG PRE = post); pre-existing defect filed separately |
| hdl-ast root schema hash | `.vu` staleness (exit 2), staged bins | unmeasured: re-pin `crates/hdl-ast/tests/schema_hash.rs`; `format_version` unchanged (§4.5.479 precedent: no sim-ir type moves) |
Byte-identity for a design with no T-typed constant: `shape_param` is `None` on every `ParamDecl` not written with
an overridable type-parameter typedef; `shape_signed(s, &None)` = `s` (`shape_bits` returns None at `?`), so
params.rs:306 returns the PRE tuple; nothing else reads the field. With a T-typed constant and NO override: `T$s`
resolves to the default's flags whose bit 0 = `tv.signed` = the parse-time `p.signed` (`signed = info.signed`), so the
same tuple again; if `T$s` failed to resolve the fallback is `p.signed`. Measured: every `_ctl`/`_uctl`/`C_*` control
(c2 45, c3 controls, c6 12, c10 13, c11 10, c12 8, c13 arrC/arrDropC/twins) byte-identical PRE vs post1 and post5;
pins 214/216 byte-identical. `.vu` bytes change for every design (one more `Option` per ParamDecl) — the hash re-pin.

## S(a)-half sets on the cut (vita-only PRE vs post5; oracle text from each set's stored/fresh runs)
- c15 = the 126 of §4.5.581's 486 lens cells (lens_diff3 v/v2/v3/v4, copied to g/c14) where post1 or post5 differs
  from PRE (post1 differs on 6: PT5c PT7c →ok, PT7 PT8 PT11 PT16 →LOUD); the other 360 byte-identical on both.
  post5: 94 loud→ok, 26 silent→ok, 6 move onto iverilog+verilator against sv2v (B02b/B02c/H13 lp+gi); 0 ok→other.
- §4.5.581 174-cell consumer matrix (s581/s2/cells): 134 byte-identical; 40 move, each onto ≥1 oracle where PRE
  matched none (SR2: PRE = verilator `0`, post5 = iverilog+sv2v `1`; LP_I/OV_I verilator-only, iverilog refuses) —
  eb9d3b69's own recorded 15 loud→value + 25 value→value.
- §4.5.581 252-cell generate-case census: 250 byte-identical; X13, X23 `def`→`a` (= sv2v; iverilog, verilator refuse).
- NOT run on the cut: §4.5.580/581's 666-variant `cmp4.py` harness, release velab/.vu corpus identity, the gate.

## Q5 fix shape and verdict
W alone (the row's shape: carry the declaration's sign per instance) is 1 routed site + a carrier, and is right on
every cell it reaches directly (152 silent→ok + 10 loud→ok over c2–c12, 0 regressions on c2/c3/c6/c10 and the pins),
but it is NOT startable alone: it moves 24 cells correct→loud and 9 correct→silent-wrong in the constant-wildcard lane
(the masked compare declines a negative left value; the decline lands in 🆕 AC's sinks / PROBE_CATALOG §4.5.581's
"range bound … takes a catch-all"). Each half alone descends: eb9d3b69 without W is PT5 loud→wrong `R=0` (§4.5.581
round 3). By measurement the two are one slice (LOOPROMPT: same-root rows, order is a measurement).
Smallest safe cut (= post5, patch g/cut_post5.patch, 12 files +248/-78):
1. hdl-ast `ParamDecl.shape_param: Option<Ident>` (re-pin `crates/hdl-ast/tests/schema_hash.rs`; format_version same).
2. parser: `ParamPrefix.shape_param` = `info.shape_param` in `parse_param_prefix`'s typedef arm when no signing
   keyword; `finish_param_assignment` puts it on the scalar `ParamDecl` and on the array arm's `NetVarDecl`;
   `None` at the 5 other `ParamDecl` constructors (+ elaborate tests/mod.rs).
3. elaborate params.rs:306: `(w, self.shape_signed(p.signed, &p.shape_param))` — the only reader a T-typed decl reaches.
4. 🆕 S (a)'s i64 half re-derived from eb9d3b69 (`const_wildcard_i64`, `wildcard_match`, `pattern_ext_fill` shared,
   `const_wildcard_masked_pre`, the masked arm leaving `const_compare_special`) with EVERY decline of
   `const_wildcard_i64` falling back to `const_wildcard_masked_pre` (`inner(..).or_else(|| masked_pre(..))`).
Moved cells on the cut vs PRE (ref = 3 agreeing oracles or 2 + a refusal; per set, c4's 14 PT designs also sit in c15):
silent→correct: c2 36, c3 69, c4 7, c5 2, c6 11 (+port_s8 2-oracle, +aliasG_s8 value only), c8 30, c9 4, c10 10, c11 4,
c12 4, arrays 5 (verilator only: arrL arrH arrInt arrW arrDrop), lens c15 26 (+6 onto iverilog+verilator vs sv2v),
matrix 25 value→value (eb9d3b69's), census X13/X23 (sv2v; iverilog and verilator refuse).
loud→value (each = the ref): c4 5, c5 1 (PT5), c8 28, c9 12, c11 3, c12 2, lens c15 94, matrix 15.
correct→anything: 0 on every set. W-alone column for contrast: correct→loud 24, correct→silent-wrong 9 (c8 c9 c11 c12),
silent→loud 18, wrong→other-wrong 1 (L_bs). Not moved, filed: ptS_ctl/ptG_ctl
(parse-time table), aliasG `b=32` (existing generate-alias row), `$signed/$unsigned` display and `$clog2(-4)` splits,
x/z override plane (row 15), struct/class/`const` louds.
Start-condition verdict: W alone NO (two descending lanes). The cut: lanes measured except the S(a) half's 666-variant
harness and release corpus identity → ER §10.2 admits it once those two are measured (or narrow the S(a) half).

## "The two declaration lanes" (row text) — identified
(1) header / body `parameter T X` bound by `bind_one_param` (params.rs; also interface headers via iface_inst and every
override channel), (2) body / generate `localparam T X` bound by instance.rs:732 / generate.rs:774 (and package.rs,
unreachable: shape None). Both size through `param_decl_width_opt`'s range arm, so one routed site serves both (c2: all
four spellings H/L/B/G moved by the same edit). A third container is the array param (`parse_array_param` →
`NetVarDecl`, already a §4.5.479 carrier type; only the parser never filled it).

## Q7 §2 / PROBE_CATALOG grep per code site
- `param_decl_width_opt`: ROADMAP:199 (`parameter signed A = 4'd10` untyped arm, BLOCKED), :200 (generate alias 32 bits
  — aliasG_* residue), :491 (dimquery-width). None is W's arm (range arm).
- `const_compare_special` / `const_str.rs` / `wildcard_eq`: ROADMAP:128 🆕 S (a) and its retry line :129 (the cut's
  second half); PROBE_CATALOG:45 (string domain, unrelated).
- `const_bound.rs`: ROADMAP:139 🆕 AC (the sink W alone feeds), :180 RECORD; PROBE_CATALOG:83, :89, :90 (§4.5.580/581
  x/wildcard decline sinks — the c9/c12 OK→WRONG mechanism).
- `const_locals` / `hdl-parser/src/params.rs`: ROADMAP:53–54 (10ⓐ/ⓑ enum labels), :107 row 15 (x/z override plane =
  the 2-state axis of a T-typed parameter), :431; PROBE_CATALOG:22, :56. No row for ptS_ctl/ptG_ctl → new row.
- `parse_array_param`, `type_param_shape.rs`, `shape_signed`, `narrow_shape_guards`, `T$s`, `const_param_fits`,
  `param_item_to_module_item`, `typedef_param_shape`: no §2 / PROBE_CATALOG line.
- REMAINING_WORK:18 (W blocks 🆕 S (a)'s i64 half) — measured here as mutual.

## Prerequisite / new rows
- none blocks the cut. New §2 row (independent of overrides): a T-typed `localparam` enters the parse-time `const_locals`
  at its raw initializer (`param_item_to_module_item`; `const_param_fits` `_ => true` on an unfoldable `[T$w-1:0]`):
  ptS_ctl `typedef struct packed {logic [X+8:0] a;}` over `localparam T X = -4`, T default `logic [7:0]`, no override:
  vita `sb=5`, iverilog/sv2v/verilator `sb=261`; ptG_ctl `g[X+5].K`: vita `gk=1`, iverilog and verilator refuse.
- Record (splits, not chased): `%0d` of `$signed(X)`/`$unsigned(X)` (iverilog `us=-4`, verilator `us=252`, explicit twin
  the same); `$clog2(-4)` of an 8-bit signed constant (iverilog/sv2v 32, verilator 8 = vita).
- If the S(a) half is not taken now: W stays BLOCKED on it (REMAINING_WORK:18 becomes mutual) — do not ship W alone.

## Artifacts
binaries (debug, s593/postN/vita): post1 W (ffe70ae3…), post2 W+eb9d3b69 (c710901b…), post3 eb9d3b69 only (18f6ab43…),
post4 W+array (a6e71b5d…), post5 cut (ebfa0e02…). patches g/: proto1.patch (W), w_carrier_post4.patch (W+array),
eb9d_i64.patch, cut_post5.patch, pindump.patch (test instrumentation, not product). cells g/c1–c15, summaries g/*.summ,
harness g/run4.sh g/runall.sh g/summ.py g/cls*.py g/vo.py.
status: DONE
