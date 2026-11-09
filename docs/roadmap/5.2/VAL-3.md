(§5.2 VAL-3 · rank — F · source W/program/reorder2-2026-11-09/PLAN.txt G2 VAL-3) (the range-bound router returns the exact typed bound):
* slot: R3.3 · L1, after VAL-2 (owner, 2026-11-09)
* scope: structure only: every output equals PRE (values, diagnostics, exit code, `.vu` and `.velab` bytes, termination, cost), PRE's differences kept as named lanes (owner, 2026-11-09)
* fix: the router (`const_range_bound_fold`, 17 calls → `range_bound_fold_ev`, `fold_x.rs:121`) returns the exact typed bound from its one fold: the value before the u64 → i64 wrap, the width, the sign, the x evidence
* fix: PRE's wrap is a named lane read by `range_facts` / `_if` / `_pair` (`const_select.rs:685` `:694` `:710`), `bound_step` (`range_rules.rs:214`) and the `Lane` variants; `RangeFacts` gains the typed bounds, read by no consumer (as its `u128` count)
* note: `RangeFacts::lazy` / `::pair` and `PAIR_SITES` unchanged, no second fold; callers untouched; size `fold_x.rs`, `const_select.rs`, `range_rules.rs` parts 1–2
* pins: `range_rules_home.rs` extended; the lane edge at 2^63 (r12 r13 r15 PRE values)
* evidence: why typed: S1v-R read the folded bounds m, l, which the router wraps from an unsigned 64-bit value to i64, so a legal design PRE gets right turned loud with a false value (W/s1v/RULINGS-premise1.txt B-1, D7 W/main_d7/2026-11-09/s1v-p1)
* witness: r12 PRE `A bw=3 l=2 r=0` = iverilog, the S1v-R prototype pR2 rc 1 naming the bound -4294967294 outside the `integer` range; r13 PRE `A bp=2` = iverilog, pR2 `-9223372036854775807`
* witness: r15 PRE `A bp=1`, pR2 R1c `element count (4294967297)` where the true count is 18446744069414584321
* witness: r11_r1w_wide_count `logic [64'hFFFF_FFFF_FFFF_FFFF : 0] w;` `$display("A bw=%0d l=%0d", $bits(w), $left(w))`: PRE rc 0 `A bw=2 l=-1` (W/s1v/plan2/filings.txt F-r11)
* oracles: iverilog `A bw=0 l=-1`, sv2v → iverilog `A bw=18446744073709551616 l=18446744073709551615`, verilator `%Error: t.sv:2:10: Value too wide for 32-bits expected in this context 64'hffffffffffffffff`
* note: risk high (fix (d)'s ground: 7 re-plans there)
* note: it frees Q-VALRULES (b)'s value rows: R1v, the bound lane, R1w and R1c (R6.1), C1 and R1x's evidence (R6.2) and the Lane track (R6.3, R6.4)
* note: the home rows' trip wires and their honest cost are written once, in §5.2 SYS-1
* source: W/program/reorder2-2026-11-09/PLAN.txt G2 VAL-3; W/s1v/RULINGS-premise1.txt
