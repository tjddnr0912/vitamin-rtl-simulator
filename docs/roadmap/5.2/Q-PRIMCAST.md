(§5.2 Q-PRIMCAST · rank ① F · source row 48 design (D1, r3 R3-1); §2 "Declined wide walk"; review r1 F3) (the region fold has no `int'` / `byte'` cast arm):
* slot: after Q-XCORE: the same file, one at a time
* V′, §2 "Declined wide walk" — `fold_region` has no prim-cast arm (size and signing casts only), so a region holding `int'(…)` or `byte'(…)` declines and keeps the unlimited i64 walk's sign: `int'(NM) ^ 64'h0` over `NM = -8'sd2` gives `fffffffffffffffe` and `byte'(N200) + 16'd0` gives `ffc8` (iverilog, sv2v and verilator `00000000fffffffe`, `00c8`; row 48 = review r1 q9_dww), and `byte'(260)` declines at the core alone
* fix: §4.5.562's design, the arm opted in per consumer with the consumer's own context and never in the shared walk (§4.5.562 was reverted after three blocking rounds, each a lane folding without its context: all-x at the bitwise, widen and work-cap sites, a class-field `fold_init` lane, a generate-scope real shadow, `{byte'(-1){1'b1}}` accepted)
* census every `fold_region` caller first
* a region holding a real-typed leaf (as Q-XFALLBACK: the binding, never the keyword) keeps PRE's route (row 48 r4: `parameter PX = 1.5;` with `int'(PX * 3) + byte'(200)` stays PRE's -51, all three; r3: `int'(((1.5 < 2) + 2) / 2)` stays PRE's 2 — iverilog, sv2v and verilator 1: the unlimited walk's cast arm `const_fn.rs:752` hands the operand to the real evaluator, which folds it whole in f64; Q-REALSPLIT, the real hand-off, parked behind row 74 — and `int'(1.5 * 3) + byte'(200)` stays -51, all three)
* exit: the two cells above at the ≤64-bit binders, `byte'(260)` in the interpreter (with row 76) and as a width (`$bits`: Q-BITSTYPE, parked behind row 74)
* must not move: §4.5.562's round cells, the 999 agreeing parity cells, the 3,059 cells
* pre-existing
