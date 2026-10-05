# s588 GROUNDING — §2 🆕 AE (constant interpreter reads never-assigned 4-state var as 0)

status: COMPLETE (Q1-Q6 answered; start condition and risks at the end)
PRE = $S/s588/pre/vita md5 e1e7e57148bb83ad35d2c4f68247a290 (release, = HEAD 2f2d3f2d)

## Q1 producer census (code read at HEAD 2f2d3f2d; file:line)

Env = `env: BTreeMap<String,i64>` (values) + `envw: ConstWidths` = BTreeMap<String,(u32 width, bool signed)> (declared shape; width 0 = unknown). No plane for x/z, no record of 4-state-ness in either map.

Writers (seed or store a value):
| # | site | what | 4-state x today |
|---|---|---|---|
| W1 | const_fn.rs:1481 `env.insert(formal, av)` | input formal <- argument (folded in CALLER env, eval_const_assign at formal width :1471) or default (callee env, depth+1 :1473); missing arg w/o default -> None | arg never x today (any x literal declines in i64 lane) |
| W2 | const_fn.rs:1492 `env.entry(k).or_insert(v)` | package constants seeded into a pkg function's env (pkg_consts: i64) | n/a (i64 consts) |
| W3 | const_fn.rs:1335 `bind_const_decl` `None => env.insert(name, 0)` | EVERY local declared without init, every kind (the docstring :1288 already flags the 4-state gap) | logic/reg/integer/time/4-state packed: should be x; seeded 0 = AE |
| W4 | const_fn.rs:1344-1354 | local with init: folds -> value; unfoldable or width unknown -> `env.remove` (UNBOUND; read = loud) | n/a |
| W5 | const_fn.rs:1506-1507 `envw.insert(name,(rw,rs)); env.entry(name).or_insert(0)` | the return variable | 4-state return: should be x; seeded 0 = AE |
| W6 | const_fn.rs:1649 Blocking `env.insert(name, v)` | whole-variable assign (rhs via eval_const_assign at target shape) | rhs never x today |
| W7 | const_fn.rs:1582-1584 `exec_const_select_write` | bit/indexed-part RMW: `cur = env.get(name).copied().unwrap_or(0)` then insert | reads seeded 0 for the unwritten bits (x2l); ALSO reads 0 for an UNBOUND local (W4's loud state) — sibling fabricated default on the same line |
| W8 | Block decls (const_fn.rs:1612) and `for (integer i=...)` | same `bind_const_decl` as W3 | same as W3 |

Readers (value reads `env.get`, membership proxies `contains_key` / `is_empty`):
| # | site | kind |
|---|---|---|
| R1 | const_fn.rs:908 Ident arm of `eval_const_env` | value; declared-but-unbound -> None (loud) |
| R2 | const_fn.rs:1213 `const_placement_wide` resolver `*env.get(name)?` | value, handed to the wide folder as BitPacked with `unk: vec![0]` (:1230-1233) |
| R3 | const_fn.rs:1518 `ConstFlow::Normal => *env.get(name)?` | return var value at body end |
| R4 | const_fn.rs:1582 select-write RMW `unwrap_or(0)` | value (fabricates 0 for absent) |
| R5 | const_fn.rs:1182, 1209; const_fn_width.rs:609, 636, 670 | membership (`env.contains_key || envw.contains_key`): shadow rule / "names a local" gate |
| R6 | const_fn.rs:1104, 1111; const_fn_width.rs:551 | emptiness proxy `env.is_empty() && envw.is_empty()` (module-scope delegation). Comment :1051 relies on "eval_const_call always seeds the return variable, so env is never empty"; envw is the conjunct with teeth (:1073) — a design that stops seeding the return var in `env` must keep envw seeded |
| R7 | const_select.rs:91 `select_span` / const_param_select_env | index/bound exprs via eval_const_env_self -> R1; base of a select resolves PARAMETERS only, so a select OF a local declines (b26/b27 measure it) |

Value consumers inside the interpreter (every arm that reads a value; all i64, x unrepresentable):
- const_fn.rs `eval_const_env` (:890): IntLit (x literal -> None, `const_eval_i64_lit` :10), Ident, `!`, reductions->self, `+ - ~` unary, Binary via `const_binop` (:118; documents "a folded const carries no x/z, so `===`/`!==`/`==?` collapse to `==`/`!=`" :112-113), Pow, shifts, Ternary (cond via eval_const_env_self, `!= 0`), `$clog2`, Call (nested, args in caller env), Concat/Replicate (R2), size Cast, selects (R7), catch-all delegation only with empty env.
- const_fn_width.rs `eval_const_env_at` (:393): fill literal, unary `+ - ~`, context ops `+ - * / % & | ^ ~^`, shifts/Pow, comparisons (`_` arm, `const_binop`), Ternary (x cond -> `selfdet_truth` only when no local named), `!`, reductions (local operand: i64 bits at declared width :674-690), `$clog2`, leaf `_` -> eval_const_env + `leaf_into_ctx`.
- Statements (const_fn.rs:1588): If/While/For conds `eval_const_env_self(..)? != 0` (:1658, 1680, 1696); Repeat count `n < 0 -> None` (:1706); Return (:1674, at ret shape); Blocking (:1648) / select write (:1579).
- Exit: `coerce_int_width(ret, rw, rs)` (:1520) — mask/sign only; no x->0 step exists (none needed while nothing is x).

Where the interpreter knows the declared type (4-state vs 2-state):
- locals: `d.kind: ast::NetVarKind` at bind_const_decl (:1322-1330, through `shape_kind(d.kind, &d.shape_param)` for a type parameter); `net_util.rs:93 net_kind_is_two_state` = Bit|Byte|Shortint|Int|Longint; 4-state = Reg|Logic|Integer|Time (netvar_kind_is_int_const const_eval.rs:80 admits exactly these 9). Kind is NOT stored in envw — dropped after the width fold.
- formals: `p.net_or_var: Option<NetVarKind>` (None = implicit logic) at :1438-1456.
- return: `f.ret_two_state: bool` (hdl-ast lib.rs:2054; "the return assignment coerces any unknown to 0") — `const_fn_ret_wsign` (inline_fn.rs:115) does not read it; `ParamType::Integer` covers both `int` and `integer` (inline_fn.rs:125).
- typedef'd / struct locals: whatever kind the parser records on NetVarDecl (a36 cell measures a packed struct local).
## Q2 consumer census

Entry: the interpreter is reached only through `eval_const_call` (const_fn.rs:1414), from three Call arms: `const_eval_in_scope` :514 / :520 (module scope, empty caller env, depth 0) and `eval_const_env` :1020 (a call nested in a body; also reached from `eval_const_env_at`'s leaf `_` arm :706). Result type `Option<i64>`: there is no x channel out. Every consumer of the module-scope folds therefore gets a value or `None` (grep counts of call sites: `const_eval_in_scope(` 99, `const_range_bound_fold(` 59, `const_int_selfdet(` 20, `const_bound_u32(` 13, `const_unsigned_selfdet(` 7, `lower_const_width_expr(` 6, `repeat_unroll_count(` 4).

Method: a POST that answers `None` for an x-bearing result routes each lane onto its EXISTING decline path. That path is measurable on PRE with a `_d` twin: the same function with `case (a) 1: t = 4'd5; endcase` in place of `if (a == 1) t = 4'd5;` (identical IEEE value; PRE declines every `case` body, §3.b const-fn-case). `_p` = PRE today (0-seeded). Cells: $S/s588/g/l (39 lanes x 2), $S/s588/g/s (12 x 2).

Storage (can the consumer hold x at all?) — $S/s588/g/c, PRE vs iverilog / verilator / sv2v:
- No <=64-bit parameter lane holds x on PRE: `localparam logic [3:0] P = 4'bxxxx;` E3009 `4'bxxxx has no constant-fold arm` (oracles `P=xxxx`), `localparam integer Q = 'x;` E3009 `parameter `Q` is declared `'x`, which this parameter model cannot hold — a parameter value has no x` (oracles `Q=x`), untyped `parameter P = 4'bxxxx` E3009, `4'b10x1` E3009, `64'bx` E3009, `time` E3009, `bit [3:0] = 4'bxxxx` E3009 (iverilog 0000). Only the >64-bit wide lane holds x (c14 `70'bx` prints 18 x = oracles). So for every <=64-bit binder IEEE's x is unrepresentable: the honest move for an x result there is loud, as for the literal twin.
- Definite x expressions already fold at module scope (c09 `4'bxxxx === 4'bxxxx` -> 1 = oracles); bitwise over x declines (c06 `& 4'b0000`, oracles 0000 — §2 🆕 H ⓕ), `+` declines (c07), x ternary declines (c10, c11), `== / <<` over x decline (c17, c18).
- The binder cannot tell 2-state from 4-state: `ParamDecl` (hdl-ast lib.rs:682) has no var_kind; `int` and `integer` are both `ParamType::Integer`, `bit [3:0]` / `logic [3:0]` / `[3:0]` are all `Implicit` + range (hdl-parser params.rs:214-383 computes `var_kind` and drops it) = §2 row 15's named prerequisite "record 2-state-ness". So no binder rule can give x->0 for `localparam int` and loud for `localparam integer`.

Per-lane today vs a decline (raw lines; `|` joins lines):
| lane | cell | PRE `_p` | PRE `_d` (= decline) | iverilog | verilator | decider -> IEEE |
|---|---|---|---|---|---|---|
| localparam logic [3:0] | l01 | P=0000 | E3009 `fx(…)` has no constant-fold arm | P=xxxx | P=xxxx | x (unholdable -> loud) |
| untyped parameter | l02 | P=0000 b=4 | E3009 | P=xxxx b=4 | P=xxxx b=4 | x -> loud |
| localparam int <- xxx1 | l03 | P=1 | E3009 | P=1 | P=X | iverilog + §6.11 x->0: 1 |
| localparam integer | l04 | P=0 | E3009 | P=X | P=X | x -> loud |
| localparam bit [3:0] <- xxx1 | l05 | P=0001 | E3009 | P=0001 | P=xxx1 | iverilog: 0001 |
| packed bound `[fx(2):0]` | l06 | b=1 | E3009 a function call that does not fold… | b=1 | %Error … isn't a two-state constant (IEEE 1800-2023 6.9.1) | split: iverilog 1 bit; verilator + §6.9.1 error |
| unpacked dim | l07 | s=1 | E3009 | s=1 | %Error 6.9.1 | split, as l06 |
| replication count (lp) | l08 | E3009 the replication `{n{…}}` has no constant-fold arm | E3009 | error: Concatenation repeat may not be undefined | %Error Replication value of < 0 or X/Z not legal | loud (both loud on PRE) |
| `V[0 +: fx(2)]` in $display | l09 | r=0 | r=1 | error: Indexed part select width must be an integral constant | %Error Internal Error | loud; PRE silent, decline silent with ANOTHER value |
| `V[fx(2):0]` | l10 | r=1 | r=1 | r=xxxxxxxx | %Error Internal | silent both (unchanged) |
| generate if (fx(2)) | l11 | E | E3010 generate-if condition is not a constant | E | T | iverilog: E (x is false) |
| generate if (fx(2)==0) | l12 | T | E3010 | E | T | iverilog: E |
| generate case (fx(2)) | l13 | C0 | E3010 | CD | CD | CD |
| generate for i<fx(2) | l14 | (0 iterations) | E3010 | error: Generate "loop" conditional expression cannot have undefined bits | (0 iterations) | loud (iverilog) |
| procedural case label | l15 | D | D | D | M | D (run-time lowering; interpreter not reached) |
| enum logic label | l16 | A=0001 | E3009 enum label `A` value is not a foldable constant | error: No function named `fx1' found | A=xxx1 | verilator: xxx1 -> loud |
| enum bit label | l17 | A=0001 | E3009 | error: No function named | %Error Enum value with X/Zs cannot be assigned to non-4-state | loud (verilator) |
| $bits(fx(2)) | l18 | E3009 `$bits(…)` has no constant-fold arm | E3009 | B=4 | B=4 | loud both (pre-existing) |
| header default | l19 | P=0000 | E3009 | P=xxxx | P=xxxx | x -> loud |
| override #(.P(fx(2))) | l20 | P=0000 | E3009 the override of parameter `P` is not a constant | P=xxxx | P=xxxx | x -> loud |
| package localparam | l21 | E3009 package parameter `P` value is not a foldable constant | E3009 | P=xxxx | P=xxxx | loud both (pre-existing) |
| instance-array prepass | l22 | E3009 `fx(…)` has no constant-fold arm [in top] | E3009 | u[0],u[1] P=xxxx | same | loud both (🆕 AD: prepass folds in the parent, which has no fx) |
| repeat (fx(2)) | l23 | n=0 | n=0 | (hangs; killed; `rc=142` with the 30 s alarm) | n=0 | §12.7.2: 0 iterations (keep) |
| #(fx(2)) | l24 | t=0 | t=0 | t=0 | t=0 | keep |
| fx(2) & 4'b0000 | l25 | P=0000 | E3009 | P=0000 | P=0000 | 0000 |
| fx(2) === 4'bxxxx | l26 | E3009 4'bxxxx has no constant-fold arm | E3009 | P=1 | P=1 | loud both (x literal) |
| localparam int = fx(2)+1 | l27 | P=1 | E3009 | P=0 | P=x | iverilog: 0 |
| int'(fx1(2)) | l28 | P=1 | E3009 | P=1 | P=X | iverilog: 1 |
| $clog2(fx(2)) | l29 | P=0 | E3009 | P=0 | P=x | iverilog: 0 |
| generate-scope localparam | l30 | P=0000 | E3009 generate-scope parameter `P` … | P=xxxx | P=xxxx | x -> loud |
| var init / CA / $display | l31-l33 | w=xxxx / w=xxxx / r=xxxx | same | xxxx | 0000 | run time, interpreter not reached (keep) |
| localparam logic [3:0] <- xxx1 | l34 | P=0001 | E3009 | P=xxx1 | P=xxx1 | x -> loud |
| (fx(2)==0) ? 1 : 2 | l35 | P=0001 | E3009 | P=00xx | P=0010 | x -> loud |
| localparam int <- xxxx | l36 | P=0 | E3009 | P=0 | P=X | iverilog: 0 |
| int = (fx1(2)==1) | l37 | P=1 | E3009 | P=0 | P=X | iverilog: 0 |
| int = !fx(2) | l38 | P=1 | E3009 | P=0 | P=X | iverilog: 0 |
| override into int param <- xxx1 | l39 | P=1 | E3009 the override … | P=1 | P=X | iverilog: 1 |
| run-time `{fx(2){2'b10}}` | s01 | E3009 a replication count of zero is only legal as a direct operand… | r=0 | error: Concatenation repeat may not be undefined | %Error Replication value … X/Z not legal | loud; DECLINE MAKES IT SILENT |
| CA `V[0 +: fx(2)]` | s02 | y=0000 | y=0001 | error: Indexed part select width… | %Error Internal | loud; decline = another silent value |
| generate-case label fx(2) | s03 | L0 | LD | LD | LD | LD |
| `V[3:fx(2)]` run time | s04 | r=xxxx | r=x | r=xxxxxxxx | %Error Internal | silent both, decline changes width |
| formal / local range holding fx | s05, s09 | E3009 | E3009 | P=1 | %Error 6.9.1 | loud both (`const_decl_wsign` declines a call) |
| localparam `V[fx(2) +: 4]` | s06 | P=0101 | E3009 | P=xxxx | %Error Internal | x -> loud |
| localparam `V[fx(2)]` | s07 | P=1 | E3009 | P=x | %Error Internal | x -> loud |
| run-time `V[0 +: fx(2)]` | s08 | r=0 | r=1 | error: Indexed part select width… | %Error Internal | loud; decline = another silent value |
| unpacked [N], N = fx(2) | s10 | E3009 an unpacked array dimension size must be a POSITIVE constant | E3009 | error: Dimension size must be greater than zero | %Error 6.9.1 | loud both |
| generate if (X), X = fx(2) untyped | s11 | E | E3009 parameter `X` … | E | T | iverilog: E |
| wire #(fx(2)) | s12 | w=1 | w=1 | w=1 | w=1 | keep |
## Q3 live 3-tool cells (interpreter-internal)

Cells: $S/s588/g/a (40), a2 (23), b (79); raw per-tool digests in a_summary.txt / a2_summary.txt / b_summary.txt; harness run4.sh (PRE; iverilog 13.0 -g2012; verilator 5.052 --binary --timing; sv2v 0.0.13 -> iverilog). Each b cell: `logic [3:0] t;` never assigned on the taken path (`if (a == 1) t = 4'd5;`, called with 2), result via `f = <expr>` into `localparam` (4-state, `%b`) or an `int` return (b80-b87).

Probe resolution / which column decides:
- iverilog refuses EVERY design whose return variable is unassigned on the taken path ("Unable to evaluate parameter P value: top.fx(32'sd2)": a01-a05, a21-a23, a28-a31, a37, a38, a45), so return-variable cells are verilator + hand-IEEE (§6.8: a 4-state variable without initializer starts x). Local-variable cells: iverilog and verilator agree on every x pattern except where noted.
- verilator is not an oracle for x->0 conversion: `int f = t;` verilator `P=X`, iverilog `P=0` (a26), `localparam int P = f(2)` verilator `P=X`, iverilog `P=0` (a32), `t && 1'b0` verilator `x`, iverilog `0` (b43), `t ? 1 : 2` verilator `0010`, iverilog and sv2v `00xx` (b20; §11.4.11 merge). Those cells are iverilog + hand-IEEE (§6.11 2-state types; §11.4.11).
- sv2v turns 2-state locals 4-state (a15 `bit` local -> sv2v `P=x`, a16, a17, a46, a49): not an oracle for 2-state; iverilog and verilator decide (0).
- verilator's constant evaluator refuses x loop conditions (b67-b69, a50 "Expecting expression to be constant"); iverilog + sv2v decide.
- iverilog reads an x index as 0 in a constant function (b28 `c[t]` -> 0000, a42 `f[t] = 1` -> 0001; verilator Internal Error): a42 is a split, kept at PRE.
- iverilog hangs on `repeat (fx(2))` with an x count (l23; killed by the alarm, `rc=142`); hand-IEEE §12.7.2 (0 iterations) + verilator `n=0` decide.

PRE census over the 142 interpreter cells: silent-wrong 80, correct 52, loud 9 (x literal b53/b58/b64, `$bits(t)` b33, prim cast b30, `$unsigned` b31, x index b28, part-select write a10, a68), no-oracle 1 (a42).

Right-by-accident (PRE = decider while the IEEE path reads x; must stay): b06 `t & 4'b0000`, b08 `t | 4'b1111`, b16 `t >> 4`, b19 `t << 4`, b21 `t ? 5 : 5`, b43 `t && 0`, b45 `t || 1`, b52 `t === t`, b56, b57, b60 `if (t == 1)`, b62 `if (t)`, b65 `if (t != 0)`, b66 `for (i < t)`, b67 `while (t)`, b69 `repeat (t)`, b71, b73, b74, b81-b83, b85, a24 x -> int formal, a26 x -> int return, a27 x -> int local, a31/a32/a33/a67 (x-bearing result into an int / bit localparam: iverilog `0` / `0001`), a57 `t & g(0)`, a64 `int k = t * 0`, a65 `if (t > 5)`, a66 `int f = t - t`.

New find on the same line (W7, const_fn.rs:1582 `unwrap_or(0)`): a bit write into a local whose initializer did not fold reads 0 for the other bits — a41 `int t = int'(2.5); t[0] = 1'b0; f = t;` PRE `P=0`, iverilog `P=2`, verilator `P=2` (not an x issue: the local is UNBOUND, the loud state W4 creates; the select write re-binds it from a fabricated 0).

Full classification (decider value; whether the IEEE function RESULT is fully known; predicted POST under shapes F0 / F1 / D defined in Q5):
| cell | PRE | decider value | decider | PRE class | IEEE fn result | F0 | F1 | D |
|---|---|---|---|---|---|---|---|---|
| a01_x2a_ret4 | P=0000 | P=xxxx | vl+IEEE | silent-wrong | x-bearing | keep | keep | silent->loud |
| a02_ret4_never | P=0000 | P=xxxx | vl+IEEE | silent-wrong | x-bearing | keep | keep | silent->loud |
| a03_x2b_retint | P=0 | P=0 | vl+IEEE | correct | known | keep | keep | keep |
| a04_d10_untyped_px | PX=0000 bx=4 | PX=xxxx bx=4 | vl+IEEE | silent-wrong | x-bearing | keep | keep | silent->loud |
| a05_d10_untyped_pi | PI=0 bi=32 | PI=x bi=32 | vl+IEEE | silent-wrong | x-bearing | keep | keep | silent->loud |
| a06_d10_ps_ctl | PS=-3 bs=6 neg=1 | PS=-3 bs=6 neg=1 | all | correct | known | keep | keep | keep |
| a07_x2c_local4 | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a08_x2i_integer | P=0 | P=x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a09_x2l_bitwrite | P=0001 | P=xxx1 | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a10_x2d_rangewrite | LOUD(E3009) | P=xx11 | ivl+vl | loud | x-bearing | keep | keep | keep |
| a11_ipwrite | P=0011 | P=xx11 | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a12_ret_bitwrite | P=0010 | P=xx1x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a13_ret_allbits | P=11100100 | P=11100100 | all | correct | known | keep | keep | keep |
| a14_x2k_bound | b=1 | LOUD | ivl+vl refuse | silent-wrong | x-bearing | keep | keep | silent->loud |
| a15_x2j_bitlocal | P=7 | P=7 | ivl+vl | correct | known | keep | keep | keep |
| a16_intlocal | P=5 | P=5 | ivl+vl | correct | known | keep | keep | keep |
| a17_byte_short_long | P=3 | P=3 | ivl+vl | correct | known | keep | keep | keep |
| a18_time_local | P=0000000000000000 | P=xxxxxxxxxxxxxxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a19_reg_local | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a20_signed_local | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a21_implicit_ret | P=0000 | P=xxxx | vl+IEEE | silent-wrong | x-bearing | keep | keep | silent->loud |
| a22_integer_ret | P=0 | P=x | vl+IEEE | silent-wrong | x-bearing | keep | keep | silent->loud |
| a23_else_only | P=0000 | P=xxxx | vl+IEEE | silent-wrong | x-bearing | keep | keep | silent->loud |
| a24_arg_2state_formal | P=1 | P=1 | ivl+IEEE | correct | known | keep | keep | keep |
| a25_arg_4state_formal | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a26_ret_2state_type | P=0 | P=0 | ivl+IEEE | correct | known | keep | keep | keep |
| a27_2state_local_from_x | P=0000 | P=0000 | ivl+IEEE | correct | known | keep | keep | keep |
| a28_x2m_order | Q=7 P=0000 | Q=7 P=xxxx | vl+IEEE | silent-wrong | x-bearing | keep | keep | silent->loud |
| a29_x2g_sibling_int | P=7 | P=0 | IEEE (7+x=x, int<-x=0) | silent-wrong | x-bearing | keep | keep | silent->loud |
| a30_x2f_callee | P=0000 | P=xxxx | vl+IEEE | silent-wrong | x-bearing | keep | keep | silent->loud |
| a31_ret_int_from_xret | P=0 | P=0 | IEEE (int<-x=0) | correct | x-bearing | keep | keep | correct->loud |
| a32_local_int_lp_xlocal | P=0 | P=0 | ivl+IEEE | correct | x-bearing | keep | keep | correct->loud |
| a33_bitlp_xlocal | P=0001 | P=0001 | ivl+IEEE | correct | x-bearing | keep | keep | correct->loud |
| a34_blocklocal | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a35_loopvar_decl | P=0011 | P=0011 | all | correct | known | keep | keep | keep |
| a36_struct_local | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a37_wide_ret70 | P=000000000000000000 | P=xxxxxxxxxxxxxxxxxx | vl+IEEE | silent-wrong | x-bearing | keep | keep | silent->loud |
| a38_ret64 | P=0000000000000000 | P=xxxxxxxxxxxxxxxx | vl+IEEE | silent-wrong | x-bearing | keep | keep | silent->loud |
| a39_init_local_ctl | P=0011 | P=0011 | all | correct | known | keep | keep | keep |
| a40_assigned_before_read_ctl | P=0011 | P=0011 | all | correct | known | keep | keep | keep |
| a41_rmw_unbound | P=0 | P=2 | ivl+vl | silent-wrong | known | silent->loud | silent->loud | silent->loud |
| a42_write_x_index | P=0001 | ? | split ivl 0001 / vl crash | no-oracle | known | keep | keep | keep |
| a43_ret4_read_before | P=0001 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a44_ret2_read_before | P=1 | P=1 | ivl+vl | correct | known | keep | keep | keep |
| a45_retbit_never | P=0000 | P=0000 | vl+IEEE | correct | known | keep | keep | keep |
| a46_typedef_bit_local | P=0010 | P=0010 | ivl+vl | correct | known | keep | keep | keep |
| a47_typedef_logic_local | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a48_typeparam_logic | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a49_typeparam_bit | P=0010 | P=0010 | ivl+vl | correct | known | keep | keep | keep |
| a50_while_uninit_i | P=0011 | P=0000 | ivl+sv2v | silent-wrong | known | keep | silent->correct | silent->correct |
| a51_accum_ret4 | P=0100 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a52_dead_self_assign | P=0011 | P=0011 | all | correct | known | keep | keep | keep |
| a55_known_bit_after_partial | P=0001 | P=0001 | all | correct | known | keep | keep | keep |
| a57_x_and_call | P=0000 | P=0000 | all | correct | known | keep | keep | keep |
| a59_self_inc_local | P=0001 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a61_static_fn | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| a62_ret2_bitwrite | P=1 | P=1 | vl+IEEE (ivl X) | correct | known | keep | keep | keep |
| a63_init_x_literal | P=0011 | P=0011 | all | correct | known | keep | keep | keep |
| a64_int_from_x_and0 | P=0 | P=0 | ivl+sv2v | correct | known | keep | keep | keep |
| a65_if_gt5 | P=0010 | P=0010 | all | correct | known | keep | keep | keep |
| a66_int_sub_self | P=0 | P=0 | ivl | correct | known | keep | keep | keep |
| a67_lp_int_retinteger | P=0 | P=0 | ivl | correct | x-bearing | keep | keep | correct->loud |
| a68_byte_ret_x | LOUD(E3009) | P=5 | ivl | loud | known | keep | keep | keep |
| b01_add | P=0001 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b02_sub | P=1111 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b03_mul0 | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b04_div | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b05_mod | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b06_and0 | P=0000 | P=0000 | all | correct | known | keep | keep | keep |
| b07_and3 | P=0000 | P=00xx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b08_orF | P=1111 | P=1111 | all | correct | known | keep | keep | keep |
| b09_or3 | P=0011 | P=xx11 | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b10_xor0 | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b11_not | P=1111 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b12_neg | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b13_pow | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b14_xnor | P=1111 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b15_shl1 | P=0000 | P=xxx0 | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b16_shr4 | P=0000 | P=0000 | all | correct | known | keep | keep | keep |
| b17_shl_by_x | P=0001 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b18_ashr | P=0000 | P=0xxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b19_shl4 | P=0000 | P=0000 | all | correct | known | keep | keep | keep |
| b20_tern_cond | P=0010 | P=00xx | ivl+sv2v (vl 0010) | silent-wrong | x-bearing | keep | keep | silent->loud |
| b21_tern_same | P=0101 | P=0101 | all | correct | known | keep | keep | keep |
| b22_tern_arm | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b23_concat | P=0001 | P=xx01 | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b24_concat_whole | P=0100 | P=01xx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b25_repl | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b26_bitsel | P=0000 | P=000x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b27_partsel | P=0000 | P=00xx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b28_index_by_x | LOUD(E3009) | P=000x | IEEE (ivl 0000, vl crash) | loud | x-bearing | keep | keep | keep |
| b29_sizecast | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b30_primcast_int | LOUD(E3009) | P=0000 | ivl | loud | known | keep | keep | keep |
| b31_unsigned | LOUD(E3009) | P=xxxx | ivl+vl | loud | x-bearing | keep | keep | keep |
| b32_clog2 | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b33_bits | LOUD(E3009) | P=0100 | all | loud | known | keep | keep | keep |
| b34_self_inc | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b35_sub_self | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b36_xor_self | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b40_redand | P=0 | P=x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b41_redor | P=0 | P=x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b42_redxor | P=0 | P=x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b43_land0 | P=0 | P=0 | ivl+sv2v (vl x) | correct | known | keep | keep | keep |
| b44_land1 | P=0 | P=x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b45_lorl | P=1 | P=1 | all | correct | known | keep | keep | keep |
| b46_lor0 | P=0 | P=x | ivl+sv2v (vl 0) | silent-wrong | x-bearing | keep | keep | silent->loud |
| b47_lnot | P=1 | P=x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b48_lt | P=1 | P=x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b49_eq_self | P=1 | P=x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b50_eq0 | P=1 | P=x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b51_ne0 | P=0 | P=x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b52_ceq_self | P=1 | P=1 | all | correct | known | keep | keep | keep |
| b53_ceq_x | LOUD(E3009) | P=1 | all | loud | known | keep | keep | keep |
| b54_cne0 | P=0 | P=1 | all | silent-wrong | known | silent->correct | silent->correct | silent->correct |
| b55_ge0 | P=1 | P=x | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b56_redand_masked | P=0 | P=0 | all | correct | known | keep | keep | keep |
| b57_redor_masked | P=1 | P=1 | all | correct | known | keep | keep | keep |
| b58_wildeq | LOUD(E3009) | P=1 | all | loud | known | keep | keep | keep |
| b59_inside | P=1 | P=x | vl+IEEE (ivl sorry) | silent-wrong | x-bearing | keep | keep | silent->loud |
| b60_if_eq1 | P=0010 | P=0010 | all | correct | known | keep | keep | keep |
| b61_if_eq0 | P=0001 | P=0010 | all | silent-wrong | known | silent->correct | silent->correct | silent->correct |
| b62_if_t | P=0010 | P=0010 | all | correct | known | keep | keep | keep |
| b63_if_not | P=0001 | P=0010 | all | silent-wrong | known | silent->correct | silent->correct | silent->correct |
| b64_if_ceqx | LOUD(E3009) | P=0001 | all | loud | known | keep | keep | keep |
| b65_if_ne0 | P=0010 | P=0010 | all | correct | known | keep | keep | keep |
| b66_for_bound | P=0000 | P=0000 | all | correct | known | keep | keep | keep |
| b67_while_t | P=0000 | P=0000 | ivl+sv2v (vl refuses) | correct | known | keep | keep | keep |
| b68_while_lt | P=0101 | P=0000 | ivl+sv2v (vl refuses) | silent-wrong | known | keep | silent->correct | silent->correct |
| b69_repeat_t | P=0000 | P=0000 | ivl+sv2v (vl refuses) | correct | known | keep | keep | keep |
| b70_return_x | P=0000 | P=xxxx | ivl+vl | silent-wrong | x-bearing | keep | keep | silent->loud |
| b71_if_ceq_self | P=0001 | P=0001 | all | correct | known | keep | keep | keep |
| b72_loop_overwrite | P=1111 | P=1111 | all | correct | known | keep | keep | keep |
| b73_if_land | P=0010 | P=0010 | all | correct | known | keep | keep | keep |
| b74_if_lor | P=0001 | P=0001 | all | correct | known | keep | keep | keep |
| b80_int_add | P=1 | P=0 | ivl (vl x) | silent-wrong | known | keep | silent->correct | silent->correct |
| b81_int_or3 | P=3 | P=3 | ivl | correct | known | keep | keep | keep |
| b82_int_and0 | P=0 | P=0 | all | correct | known | keep | keep | keep |
| b83_int_concat | P=1 | P=1 | ivl | correct | known | keep | keep | keep |
| b84_int_eq0 | P=1 | P=0 | ivl | silent-wrong | known | silent->correct | silent->correct | silent->correct |
| b85_int_shl1 | P=0 | P=0 | ivl | correct | known | keep | keep | keep |
| b86_int_not | P=-1 | P=-16 | ivl | silent-wrong | known | keep | silent->correct | silent->correct |
| b87_int_tern | P=2 | P=0 | ivl (vl 2) | silent-wrong | known | keep | silent->correct | silent->correct |

F0 {'keep': 137, 'silent->correct': 4, 'silent->loud': 1}
F1 {'keep': 132, 'silent->correct': 9, 'silent->loud': 1}
D {'correct->loud': 4, 'keep': 58, 'silent->correct': 9, 'silent->loud': 71}
PRE classes {'silent-wrong': 80, 'correct': 52, 'loud': 9, 'no-oracle': 1} total 142

## Q4 lane table (ER §10.2)

Shared functions the fix edits or routes into: `eval_const_call` (const_fn.rs:1414; callers = the 3 Call arms), `bind_const_decl` (:1315; 2 decl sites :1503, :1613), `exec_const_stmt` (:1588), `exec_const_select_write` (:1554), and — routed into, not edited, under F0 — `fold_bits_at` / `fold_region` (const_wide.rs:493 / :510) through a new resolver modelled on `const_placement_wide`'s (const_fn.rs:1156). F1 would add opt-in arms to `fold_region` (ordering, arithmetic, bitwise, ternary merge) passed only from the interpreter's resolver path: every module-scope lane opted out (byte-identical).

Two shapes are scored (Q5): F = frozen exit (an x-bearing result returns PRE's value; only a fully-known 4-state result replaces it); D = decline at the exit (the row's literal reading). Marks: keep / moves (direction) / unmeasured.

A. Consumer lanes for an x-valued RESULT (fx / fx1; $S/s588/g/l, s, c):
| lane | PRE | iverilog | verilator | IEEE / decider | F | D |
|---|---|---|---|---|---|---|
| 4-state <=64-bit localparam / parameter / header default / override / generate-scope lp (l01, l02, l04, l19, l20, l30, l34) | 0-based value | x | x | x, unholdable -> loud | keep (silent-wrong residue) | silent->loud |
| 2-state localparam int / bit / override into int (l03, l05, l36, l39, a32, a33, a67, a31) | 0-based = x->0 | value | X (not an oracle for x->0) | §6.11 x->0: PRE right | keep | correct->loud (8) |
| packed bound / unpacked dim (l06, l07) | 1 bit | 1 bit | error §6.9.1 | split: §6.9.1 + verilator error vs iverilog 1 | keep | moves to loud (verilator+IEEE side) |
| replication count lp / run time (l08, s01) | loud / loud | error | error | loud | keep | l08 keep; s01 loud->SILENT (`r=0`) |
| `+:` width (l09, s02, s08) | r=0 / y=0000 / r=0 | error | Internal Error | loud | keep | silent->silent' (r=1, y=0001, r=1) |
| `[m:l]` (l10, s04) | r=1 / r=xxxx | xxxxxxxx | Internal Error | x | keep | l10 keep; s04 silent->silent' (r=x) |
| generate if whole call / untyped X (l11, s11) | E | E | T | x is false: E | keep | correct->loud (2) |
| generate if composite (l12), generate case (l13), generate for (l14), gen-case label (s03) | T / C0 / 0 iter / L0 | E / CD / error / LD | T / CD / 0 iter / LD | E / CD / loud / LD | keep | silent->loud (3); s03 silent->correct |
| enum labels (l16, l17) | A=0001 | refuses (no function fd in enum) | xxx1 / error | x / error | keep | silent->loud (2) |
| composite module-scope: `&0` (l25), `int'()` (l28), `$clog2` (l29) | 0000 / 1 / 0 | same | 0000 / X / x | PRE right | keep | correct->loud (3) |
| composite module-scope: `+1` int (l27), `==1` (l37), `!` (l38), `?:` (l35) | 1 / 1 / 1 / 0001 | 0 / 0 / 0 / 00xx | x / X / X / 0010 | PRE wrong | keep (residue) | silent->loud (4) |
| index from call (s06, s07) | 0101 / 1 | xxxx / x | Internal Error | x | keep | silent->loud (2) |
| already loud on PRE (l18 $bits, l21 package lp, l22 instance-array prepass, l26 x literal, s05/s09 call in formal/local range, s10) | loud | value | value / error | — | keep | keep |
| run time, interpreter not reached (l15, l23 repeat, l24 delay, l31-l33, s12) | = iverilog | | | | keep | keep |

B. Lanes a MOVED (fully known) result flows through under F — function `fd`: `if (t == 4'd0) fd = 1; else fd = 2;` with `t` never assigned; IEEE / iverilog / verilator `2`, PRE `1` ($S/s588/g/m):
| lane | cell | PRE | iverilog | verilator | sv2v | F |
|---|---|---|---|---|---|---|
| localparam logic [3:0] | m01 | P=1 | P=2 | P=2 | P=2 | silent->correct |
| untyped parameter | m02 | P=1 b=4 | P=2 b=4 | P=2 b=4 | same | silent->correct |
| localparam int | m03 | P=1 | P=2 | P=2 | P=2 | silent->correct |
| packed bound | m04 | b=2 | b=3 | b=3 | b=3 | silent->correct |
| generate if | m05 | E | T | T | T | silent->correct |
| replication count | m06 | E3009 the replication `{n{…}}` has no constant-fold arm | 00000011 | same | same | keep (loud; count fold never calls the interpreter) |
| `+:` width | m07 | r=1 | r=01 | r=01 | r=01 | silent->correct |
| override | m08 | P=1 | P=2 | P=2 | P=2 | silent->correct |
| header default | m09 | P=1 | P=2 | P=2 | P=2 | silent->correct |
| `pk::fd` | m10 | P=1 | P=2 | P=2 | P=2 | silent->correct |
| `import pk::*` | m11 | P=1 | P=2 | P=2 | P=2 | silent->correct |
| `$unit` function | m12 | P=1 | P=2 | P=2 | P=2 | silent->correct |
| nested (`g = fd(a)+1`) | m14 | P=2 | P=3 | P=3 | P=3 | silent->correct |
| enum label | m15 | A=1 B=2 | error: No function named `fd' found | A=2 B=3 | A=2 B=3 | silent->correct (verilator + sv2v) |
| generate case | m16 | C1 | C2 | C2 | C2 | silent->correct |
| generate for | m17 | I0 | I0 I1 | I0 I1 | I0 I1 | silent->correct |
| instance-array prepass | m18 | E3009 `fd(…)` has no constant-fold arm | P=2 x2 | P=2 x2 | same | keep (🆕 AD) |
| procedural repeat (unrolled through `repeat_unroll_count`) | m19 | n=1 | n=2 | n=1 | n=2 | silent->correct (iverilog + sv2v + IEEE; verilator 0-initialises) |
| delay / run-time case label / run-time call | m20, m24, m25 | t=2 / M / r=2 | t=2 / M / r=2 | t=1 / D / r=1 | = iverilog | keep (vita's run time is already right: r=2 — the interpreter contradicts vita's own engine) |
| generate-scope lp | m21 | P=1 | P=2 | P=2 | P=2 | silent->correct |
| unpacked dim | m22 | s=1 | s=2 | s=2 | s=2 | silent->correct |
| size cast `fd(2)'(8'hFF)` | m23 | 00000001 | 00000011 | same | same | silent->correct |
| staged (vcmp/velab/vrun, PRE sep bins) | b61, a07, b84, a01 | P=0001 / 0000 / 1 / 0000 | = monolithic PRE | | | same interpreter at velab: moves as monolithic |
| composite corrected + frozen (`fd(2) + fx(2)`) | m26 | P=0001 | xxxx | xxxx | xxxx | silent->silent' (0010): the one F trade found |
| nested call with an x argument to a 4-state formal | m28 | P=0001 | 0010 | 0010 | 0010 | keep under F0 (a call inside a tainted expression declines -> legacy) |
| tainted select `t[3:2] == W` | m27 | E3009 | 0010 | 0010 | 0010 | keep (legacy loud short-circuits) |

C. Population (probe binaries = HEAD + logging only, release, md5 2249f265256c275bcf5f3eda7b0d10f0 for a/a2/b/l/c and 4d5e17eba66eab7172f25593a4a8cb88 (+ call counter) for m/m2/s and the corpus; outputs byte-identical to PRE on all 289 cells: `diffs=0` both runs):
- corpus (`corpus-runner run --reps 1`, 11/11 workloads, grades identical to the pinned table): interpreter calls 4 (verilog-axi `calcBaseAddrs` x4), reads of a never-assigned 4-state var 0, select writes into an unbound name 0.
- suite (`cargo nextest run --workspace --locked --no-fail-fast` on the probe tree, VITA_AE_PROBE set): `Summary [  41.379s] 9048 tests run: 9047 passed, 1 failed, 15 skipped`; the 1 failure is my harness, not the probe: `corpus-runner::manifest every_uncommitted_manifest_path_is_gitignored` — `git check-ignore failed (exit status: 128): fatal: pathspec 'bench/aes/src/' is beyond a symbolic link` (I symlinked bench/*/src into the worktree for the corpus run). Tainted reads over the whole suite: 0 (no log file created). Propagation proven: `cli --test const_fn_decl_bind` alone logged 419 interpreter CALL lines and 0 tainted reads.
- So no corpus design and no suite test reads a never-assigned 4-state variable in a constant function: every shape that only changes tainted calls (F or D) is byte-identical on the whole suite and corpus; the moved population is the new cells only.

Unmeasured lanes: none for F (A: every lane keeps by construction — an x-bearing result returns PRE's value through the unchanged `Option<i64>` channel — and is measured anyway; B: every consumer kind measured on PRE and oracles). For D: none unmeasured, but 13 correct->loud + 2 split, 1 loud->silent, 4 silent->silent' (A).
## Q5 proposed fix shape

Shape F (recommended; "frozen exit"):
1. Legacy run first, today's code verbatim, plus a taint flag set by a READ (R1 Ident :908, R2 placement resolver :1213, R3 return read :1518, R4 select-write RMW :1582) of a var seeded by W3 (4-state kind via `net_kind_is_two_state(shape_kind(..))`, no initializer) or W5 (return var, `!f.ret_two_state`) and not yet wholly assigned. These are exactly the probe's sites.
2. Legacy `None` -> `None` (nothing PRE refused can fold). Untainted -> legacy value (no other code runs).
3. Tainted with legacy `Some(v)` -> re-run the body in 4-state mode: a parallel `unk: BTreeMap<String,u64>` plane (absent = known); x seeded for the W3/W5 vars; an expression naming an unknown-bearing var evaluates through `fold_bits_at(rhs, ctx = target width, resolver)` (resolver = `const_placement_wide`'s, with `unk` filled); untainted expressions keep the i64 walk; If/While/For take a branch only on a known 1 bit (`bp_truth == Some(true)`; §12.4: x is false); Repeat with an unknown count runs 0 times (§12.7.2); a store into a 2-state target / formal / return converts x->0 (§6.11); into a 4-state target keeps the planes masked to width; a select write with a known span clears those unknown bits, an unknown index declines; a call, an x/z literal, `$bits`, a prim cast or any arm `fold_region` declines on (ordering, arithmetic, bitwise, ambiguous ternary under F0) declines the re-run.
4. Exit: a fully-known 4-state result replaces `v`; an x-bearing or declined re-run returns `v` (PRE verbatim). The x-bearing exit is frozen because every rule that changes it descends (D, below), and the binder that would decide x->0 vs loud cannot tell `int` from `integer` (row 15's prerequisite).
5. W7 sibling (same line): a select write into an UNBOUND name declines (`env.get(name)?` instead of `unwrap_or(0)`): a41 silent->loud.
F1 = F0 + opt-in `fold_region` arms (ordering -> x, `+ - * / % **` / unary minus with any unknown -> all x, per-bit `& | ^ ~^ ~` tables = §2 🆕 H ⓕ, §11.4.11 ambiguous-ternary merge) reachable only from the interpreter's resolver path.

Byte-identity (code path, not "should"):
- Every call that reads no seeded-x var returns the legacy value through the identical path; the only added work is the flag check at R1-R4. Corpus: 4 interpreter calls, 0 tainted (probe). Suite: below.
- Every tainted call either returns the legacy value (re-run x-bearing / declined) or a fully-known IEEE value; no path returns `None` where PRE returned a value except W7's unbound select write (a41), and no path returns a value where PRE returned `None` (legacy `None` short-circuits). So no consumer lane sees a new decline: 🆕 AC's silent fallbacks, the 2-state binders and the composite module-scope folds are untouched by construction.
- 2-state paths: a 2-state local / formal / return is never seeded x, so it never sets the flag (a03, a15-a17, a44-a46, a49, a62 and the pin `int x; g = x + 1;` -> 1 in const_fn_decl_bind.rs:279 stay on the untainted path). Assigned-before-read paths clear the seed at W6 before any read (a39, a40, a13 via per-bit RMW, a35, a52).

Moved-cell counts (interpreter cells Q3, 142; lane cells Q4):
| shape | silent->correct | silent->loud | correct->loud | silent->silent' | loud->value | loud->silent |
|---|---|---|---|---|---|---|
| F0 | 4 interp (b54, b61, b63, b84) + 18 lanes (m01-m05, m07-m12, m14-m17, m19, m21-m23) | 1 (a41, W7) | 0 | 1 (m26 corrected + frozen in one composite) | 0 | 0 |
| F1 | 9 interp (+ a50, b68, b80, b86, b87) + same 18 lanes | 1 | 0 | 1 (m26) | 0 | 0 |
| D | 9 interp + s03 | 71 interp + 18 lane (l01, l02, l04, l12-l14, l16, l17, l19, l20, l27, l30, l34, l35, l37, l38, s06, s07) | 13 (a31, a32, a33, a67, l03, l05, l11, l25, l28, l29, l36, l39, s11) + 2 split (l06, l07) | 4 (l09, s02, s04, s08) | 0 | 1 (s01) |
F leaves every x-bearing exit silent-wrong exactly as PRE: the row's headline cells x2a / x2c / x2i / x2l / d10 / x2m / x2n (a01, a02, a04, a05, a07-a09, a11, a12, a18-a23, a25, a28, a30, a34, a36-a38, a43, a47, a48, a51, a59, a61 and 46 b cells) and x2k (a14) — the residue "AE-b", BLOCKED on: (P1) §2 row 15 "record 2-state-ness" (ParamDecl hdl-ast lib.rs:682 has no var_kind; needed so a 2-state binder converts x->0 while a 4-state one goes loud); (P2) §2 🆕 AC's sinks refusing an x-valued call instead of falling back (s01, s02, s04, s08, l09; the PROBE_CATALOG §4.5.580 / §4.5.581 rows are the same sinks); (P3) a module-scope 4-state carrier for a call result (l25 `& 0`, l28 `int'()`, l29 `$clog2`, l11 / s11 generate-if truth, m26).
## Q6 §2 / PROBE_CATALOG rows on the touched sites

ROADMAP (grep `const_fn`, `envw`, `bind_const_decl`, `wide_eq_with_unknowns`, `fold_region`, `const_fn_ret_wsign`, `const_param_select_env`):
- §2 Constant domain: "`integer x; g = x + 1;` is 1 (iverilog x); iverilog; OPEN" — the integer-local half of 🆕 AE itself (W3); under F it stays (x-bearing exit); fold into AE-b or mark DUP.
- §2 🆕 H ⓔ "loud constant reductions … after a const-function local assignment (`envw`)" (reduction arm const_fn_width.rs:670, membership R5) and ⓕ "a bitwise op over x/z (`const_wide.rs`: per-bit 4-state tables)" = F1's bitwise rule (`fold_region` :647-676).
- §2 Constant domain: "A non-zero-LSB local in a concat is read by position (`envw` has no LSB)" — R2's resolver, which F's 4-state route reuses (inherits the same positions for known bits).
- §2 Constant domain: "A formal shadowing a parameter is selected as the parameter (`const_param_select_env`)" — R7.
- §2 Constant domain: "The interpreter reads an `int unsigned` return signed (`const_fn_ret_wsign`)" — W5's `(rw, rs)`.
- §2 Constant domain: "A count over `4'sbx000 == 8'sd5` collapses to 0 (`wide_eq_with_unknowns`)" — the eq arm F0 routes x through (a signed x-MSB operand declines in `widen_to`; the count defect is `fold_count`'s).
- §2 Constant domain: "Masked overflow … `const_fn_width.rs` `checked_mul`", "Placement / cast fold residue (… concat x/z …)" — adjacent arms, untouched by F0.
- §2 start-order 🆕 AC (P2), 🆕 AD (l22 / m18 prepass loud), row 15 + its "same plane" sub-line ("a declaration holding x is E3009") (P1), 🆕 AE (this row); §3.b `const-fn-case`, `const-fn-systask`, `unique-const-fn` (BLOCKED on AE).
PROBE_CATALOG: §4.5.580 rows "An x-valued compare used as a size … a declined x value is not refused" (replication count, part-select bound) and §4.5.581 "An x-valued `==?` as an indexed part-select WIDTH is silent" / "A range bound or array dimension whose constant fold declines on an x/z wildcard compare takes a catch-all at exit 0" — the same sinks D would feed (P2).
New (not on file): W7 a41 — a select write into a local whose initializer did not fold reads 0 for the unwritten bits (PRE `P=0`, iverilog / verilator `P=2`), const_fn.rs:1582.

## Start condition and open risks

Start condition (ER §10.2, zero unmeasured lanes): HOLDS for shape F (every consumer lane measured on PRE + oracles: Q4 A keeps by construction, Q4 B moves value-level; suite + corpus population measured = 0 tainted). DOES NOT HOLD for the row's literal shape D: lanes are measured but D descends (13 correct->loud + 2 split, 1 loud->silent, 4 silent->silent'); its x-bearing-exit half (AE-b) needs P1 (row 15 "record 2-state-ness"), P2 (🆕 AC sinks refuse an x-valued call), P3 (module-scope 4-state carrier for a call result) first. F does not close the row's headline cells; it closes the interpreter-internal decisions (Q5 counts) and the W7 sibling.

Open risks:
1. F's 4-state route is `fold_bits_at`, a different walk from `eval_const_env_at` for the KNOWN parts of a tainted expression (region width/sign two-pass; the placement resolver's `(32, false)` guess for a module name with no `param_meta`). A known result could differ from the i64 walk for the same known operands; mitigation: decline the re-run when a tainted expression names a module parameter without declared provenance, and census tainted expressions mixing parameters (m27 shows the select form is already loud on PRE).
2. Recursion: a tainted recursive function must not double-run at every level (2^depth); nested calls inside the 4-state re-run must be 4-state-only (single run), giving O(depth^2) worst case.
3. m26: a composite consumer holding a corrected call and a frozen x-bearing call moves between two wrong values (PRE 0001 -> F 0010; oracles xxxx). Only avoidable with P3 or by freezing every call of a fold that holds a frozen one (no fold-scope boundary exists in `const_eval_in_scope`).
4. F0 vs F1: F1's extra rules live in shared `fold_region`; opt-in keeps module-scope lanes byte-identical, but the rules overlap §2 🆕 H ⓕ (bitwise tables) — landing F1 there first, or as an opt-in parameter, is a planner call.
5. a42 (x index on a select write) is an oracle split (iverilog 0001 = PRE; verilator Internal Error); F declines the re-run there, keeping PRE.
6. IEEE clauses cited (§6.8 x initial value, §6.11 2-state x->0, §11.4.11 merge, §12.4 x condition false, §12.7.2 x repeat count = 0, §6.9.1 two-state range) are recalled, not re-read; each is backed by the iverilog / verilator lines in Q3/Q4.
7. iverilog hangs on `repeat (x)` (l23): any harness that runs iverilog on such a cell needs the alarm guard (run4.sh now wraps vvp / the verilator sim with `perl -e "alarm 30; exec @ARGV"`).
8. W7 (a41) fix is a new decline: measure it on the suite before review (the probe logged 0 RMW-UNBOUND over suite + corpus, so it is suite-neutral).
