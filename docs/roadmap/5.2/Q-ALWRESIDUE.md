(§5.2 Q-ALWRESIDUE · rank ① · source Q-ALWINTRA reviews r1–r3: r2 lens-diff D5 (cells/d5), r3 lens-sound S3) (defects Q-ALWINTRA's reviews found that stand without its rule, one per clause):
* slot: past the last numbered slot
* slot: starts only on a track whose next C row cannot start
* scope: what Q-ALWINTRA's reviews raised that stands without that row's rule: every clause is PRE 719e37d2 text, one root each, no page or corpus line filed with any of them
* a cell is under W/qalwintra/r2/lens-diff/cells/d5 unless named.
* **Q-VIFDLY**: a delay on a virtual-interface blocking assign is dropped, silent: `elaborate/src/stmt_main.rs:78-84` returns on a `vif_handles` target before any delay is lowered: `initial begin vif = #1 i0; $display("T %0d", $time); $finish; end` (W/qalwintra/r3b/lens-sound/cells/c2/vif_init_twin) PRE `T 0`, verilator 5.052 `T 1`, iverilog 13.0 `t.sv:3: syntax error` `t.sv:3: error: Invalid module item.`, sv2v `` t.sv:3:13: Parse error: missing expected `endmodule` ``
* IEEE 1800-2017 §9.4.5.
* **Q-STRLEN32**: `Value::from_str_bytes` (`sim-engine/src/value.rs:943`) `(bytes.len() as u32) * 8` wraps at 512 MiB: rc 101 once, `index out of bounds: the len is 1 but the index is 1` at `value.rs:946`, on the parked Q-ALWINTRA build, `s = {s, s};` in a function (W/qalwintra/r3b/lens-sound/cells/b2/fn_mod_s_dbl), sim-engine untouched there
* PRE witness UNVERIFIED (the 2 GB cap came first, 4 runs).
* **Q-PKGDLYSCALE**: a package task's `#1` and an interface body's run at the lowering scope, against IEEE 1800-2017 §3.14: `crates/cli/src/frontend.rs:434` resolves a \`timescale for modules only.
* A package task `#1` under `` `timescale 1us/1ns `` called from a module under `` `timescale 1ns/1ns ``, `initial begin t; $display("T %0d", $time); $finish; end` (W/qalwintra/r2/lens-diff/cells/d4/w_init) PRE `T 1`, sv2v `T 1`, iverilog 13.0 = verilator 5.052 `T 1000`
* an interface body under `` `timescale 1us/1ns ``, `always begin #1 x = ~x; k = k + 1; end` (d4c/ifc_stmt) PRE = sv2v = verilator `END 1500 k=1499`, iverilog `END 1500 k=1`.
* **Q-TASKNET**: a task body's net binds at the caller's generate scope: `task t; y = #3 ~y; endtask` at module level, `initial begin t; t; t; end` in a generate block that declares its own `y` (W/qalwintra/r2/lens-diff/cells/gn/net_initial) PRE `END 23 y=0 g.y=1 n=0`, iverilog = sv2v = verilator `END 23 y=1 g.y=0 n=0`.
* **Q-TIMEUNIT**: `timeunit` / `timeprecision` at compilation-unit and module scope are E2002, 16 of 16 cells U_* (W/qalwintra/r3b/lens-sound/cells/b7), `t.sv:1:1: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected 'module', found identifier 'timeunit'`, the manual says Absent
* `always begin y = #2 ~y; n = n + 1; end` under them: iverilog 13.0 = sv2v = verilator 5.052 `END 23 y=1 z=0 n=11` on 12 of 16
* U_cu_bad U_mod_bad (`timeunit 1ps; timeprecision 1ns;`) iverilog `t.sv:1: error: A timeprecision is missing or is too large!`, verilator `%Error: Internal Error: t.sv:5:38: ../V3EmitCFunc.h:1409: TimeQ is less than 1, will result in division by zero`, sv2v `END 23 y=1 z=0 n=11`
* U_cu_unit U_mod_unit (no `timeprecision`) iverilog the same error, verilator and sv2v `END 23 y=1 z=0 n=11`.
* Parsing them reopens the `#1` is at least one tick premise of W/qalwintra/replan/REPLAN.txt:29.
* **Q-PKGENABLE**: a scoped task enable `p::t();` is E2002: `always begin p::t(); n = n + 1; end` with a 1us package task `#1` (d4c/scoped_paren) `t.sv:9:17: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected '=' or '<=' after lvalue, found '::'`, verilator `END 1500 n=1`, sv2v `END 1500 n=1499`, iverilog `t.sv:9: syntax error` `t.sv:9: error: Malformed statement`
* 003's `pkg::f(...)` scoped routine call is Supported, for a function.
* **Q-REP2P31**: `repeat (n)` runs no iteration from 2^31 up: `initial begin repeat (32'h8000_0000) x = #1 ~x; n = n + 1; end` beside `initial #22` (r6_initial_twin_repeat_2p31; `lower_repeat`, `elaborate/src/stmt_flow.rs:1113`) PRE `END 22 1` = verilator 5.052, iverilog 13.0 = sv2v `END 22 0`
* IEEE 1800-2017 §12.7.2 runs the count
* r5_rep_tick_big_value_only, its `always begin` spelling, PRE W3056 `END 22 1`, iverilog = sv2v `END 22 0`, verilator `%Error-DIDNOTCONVERGE: Settle region did not converge after '--converge-limit' of 10000 tries`.
* **Q-FORKBUDGET**: no budget bounds forks at one time value: `always begin fork #5 y = ~y; x = ~x; join_any disable fork; end` (W/qalwintra/r3b/lens-sound/cells/b7/s21) PRE killed at 28 s, 33 MB, `#22 $finish` not reached, no oracle has a value
* a pass costs more each time (r1 lens-sound e29, r1's build, no PRE twin: 37.6 MB at 140 s)
* with a live clock it is quadratic: `always begin fork y = #5 ~y; x = #1 ~x; join_any disable fork; n = n + 1; end` (b7 d1, the parked build) 0.32 / 1.18 / 4.60 / 23.83 s at 20,000 / 40,000 / 80,000 / 160,000 passes, RSS 9 to 22 MB, value right
* statement twin on PRE 22.93 s at 160,000 (r3 lens-sound N2; its table row not found)
* the fads series on a PRE twin and iverilog's 3 M passes in 3.07 s: not re-run, UNVERIFIED (N3).
* The rest of the name (a37, m15) is §5.b NBA-GROWTH.
* **Q-ALWHIER**: an `always` whose timing sits in a task reached by a hierarchical, interface, package-nested or recursive enable runs one pass under W3056: `always fork u.tk; join_any` (fk14_hier_any, `task tk; clk = #5 ~clk; endtask` in `u`) PRE `T 5 1`, iverilog 13.0 `T 5 1 / T 10 0 / T 15 1 ..`, verilator 5.052 `T 0 0 / T 5 1 / T 10 0 ..`
* x13 PRE `top.u[0] Y 5 1`, iverilog = sv2v `top.u[0] Y 5 1 / top.u[0] Y 10 0 / top.u[0] Y 15 1 ..`
* the same W3056 one pass on u02 u05 x01 t_iface_call t_iface_hier tr_d_hr pk_nested_star tr11 tr15 tr33 mr03 w_recursive_arg (W/qalwintra/r3b/EXPECT.tsv)
* the module-task twin is PROBE_CATALOG §4.5.597.
* **Q-ARMEDGE**: an arm that reaches `@(posedge c)` with `c` already 1 catches the edge it should wait past: `always #2 begin c = ~c; $display("C %0t %b", $time, c); end`, `initial begin #2; $display("R %0t %b", $time, c); @(posedge c); $display("W %0t", $time); end` (e1_arm_after_edge) PRE `C 2 1 / R 2 1 / W 2`, iverilog = sv2v `C 2 1 / R 2 1 / C 4 0 / C 6 1 / W 6`, verilator `R 2 0 / C 2 1 / W 2`.
* **dB1**: native orders an arm's wake against its parent differently from vm and interp: `always fork begin @(posedge c) x = ~x; n = n * 2; … end begin @(posedge c) w = ~w; n = n + 1; … end join` (o8_ctl_stmt_mul) PRE native `END 23 158`, PRE vm = interp = iverilog = sv2v = verilator `END 23 127` (r1 lens-diff REPORT:24; `sim-engine/src/native/run.rs:1682` `tie: w.proc` in `fire_waiters`)
* W/qalwintra/replan/xd/SUMMARY.txt has a fix at native = vm = interp on 21 of 21 cells, iverilog 19 of 21, 3 PRE cells (ov_stmt_cap, o8, o11) move onto vm, interp and iverilog (W/qalwintra/replan/bin/vita-xd md5 6c874d0bb2f1e336217bba0d8064a8c3).
* **Q-FINALFORK**: a `fork` with no timing in `final` drops the rest of the block at exit 0: `final begin fork z = 1; join $display("F %0d %b", $time, z); end` (fn21) prints nothing, iverilog 13.0 = sv2v = verilator 5.052 `F 10 1`
* `join_any` fn22, iverilog = verilator `F 10 1`, sv2v `` t.sv:5:27: Parse error: missing expected `join` ``
* fn26 iverilog = verilator `F 10 1 1`
* IEEE 1800-2017 §9.2.3.
* **Q-FINALNBA**: a non-blocking assignment in `final` is accepted, PRE `F 10 0` (fn09 `y <= x;`, f02, f06, fn10) where iverilog and verilator refuse: fn09 iverilog `t.sv:5: error: final procedures cannot have non blocking assignment statements.`, verilator `%Error-FINALDLY: t.sv:5:17: Non-blocking assignment '<=' in final block`
* f02 verilator `%Error: t.sv:5:20: Delays are not legal in final blocks (IEEE 1800-2023 9.2.3)`
* fn10 iverilog `t.sv:6: error: final procedures cannot enable/call tasks.`
* f06 PRE the same, no oracle text kept.
* **Q-MUTREC**: mutual recursion of automatic tasks skips the timing: `task automatic a(int d); if (d > 0) b(d - 1); endtask`, `task automatic b(int d); if (d > 0) a(d - 1); else #5 k = k + 1; endtask`, `initial begin a(3); $display("D %0d %0d", $time, k); end` (mr01) PRE `D 0 0`, iverilog = sv2v `D 5 1 / T 5 1`, verilator `T 0 0 / D 5 1 / T 5 1`
* an `always` of such a pair (tr02) PRE W3056, no output, verilator `T 0 0 / T 5 1 / T 10 2`
* root not found.
* **Q-NESTBIND**: a task's nested enable binds from the caller's scope, not the task's: `task inner; clk = #5 ~clk; endtask task outer; inner; endtask` with a generate block declaring its own `task inner; clk = ~clk; endtask` and `always outer;` (tr07) PRE W3056 `T 0 1`, iverilog = sv2v `T 5 1 / T 10 0 / T 15 1 ..`, verilator `T 0 0 / T 5 1 / T 10 0 ..`
* `task_enable_blocker` (`elaborate/src/const_level_header.rs:433`) documents the disagreement
* a `$unit` task enabling a name a module also declares (W/qalwintra/r1/lens-sound/d1/u30) PRE `LOCAL T2 0`, iverilog = verilator `UNIT T2 5 / UNIT T2 10 / UNIT T2 15 ..`.
* **Q-FORKNULL**: a `;` statement counts as a `fork` arm: `initial begin fork #5 y = 1; ; join_any $display("A %0t %b", $time, y); end` (v03) PRE `A 0 0`, iverilog 13.0 = verilator 5.052 `A 5 1`, sv2v `` t.sv:3:34: Parse error: missing expected `join` ``
* v06b PRE `A 0 0`, iverilog = verilator `A 3 1`
* a `begin end` arm (v04) PRE `A 0 0`, iverilog `A 5 1`, verilator `A 0 0`
* IEEE 1800-2017 §9.3.2 `statement_or_null`, the text does not settle it.
* **Q-FINISHBODY**: a body with no timing that reaches `$finish` is not inert: `always begin n = n + 1; if (n == 5) begin $display("D %0d", n); $finish; end end` (u06) PRE W3056, no output, verilator 5.052 `D 5` then `%Error-DIDNOTCONVERGE: Settle region did not converge after '--converge-limit' of 10000 tries`, iverilog 13.0 `t.sv:3: error: always process does not have any delay.`
* a candidate: a terminating loop is not a suspension.
* **Q-ALWSTAR**: `always y = @(*) z;` (e09, `@*` e10) lowers an empty read set: W3056 twice, the second "in-body @(*) reads no nets; it can never wake", no output, verilator 5.052 `Y 0 0 / Y 7 1 / Y 14 0`, iverilog 13.0 `t.sv:6: syntax error`.
* **Q-DLYCALL**: `always clk = #f(3) ~clk;` (d19), a delay holding an unparenthesized call, is accepted, PRE W3056 `T 5 1 / A n=1 clk=1`, iverilog 13.0 `t.sv:4: syntax error`, sv2v `t.sv:4:22: Parse error: unexpected token '~' (Sym_tildy)`, verilator 5.052 `%Error: t.sv:4:22: syntax error, unexpected '~', expecting ';'`
* IEEE 1800-2017 A.6.5.
* **Q-TIMESAT**: the clock saturates, and every later crossing is zero time: `always begin #1 x = ~x; w <= ~w; n = n + 1; #9223372036854775807 y = 1; end` with a 1 Mbit `w` (W/qalwintra/r2/lens-sound/cells/a7/t_s1) PRE killed at 2 GB in 0.61 s, no fatal
* the header-less spelling s1_sib_wide PRE W3056 `F 1 9223372036854775808`, iverilog 13.0 `$finish called at 9223372036854775809` `F 100 ...`, sv2v `F 100 199`, verilator %Error on the time
* the twin itself on no oracle
* §5.b DELAY-CLAMP is the neighbour
* source: N1–N4 (W/qalwintra/r3b/lens-sound/REPORT.txt), W/qalwintra/r3b/EXPECT.tsv; funnel: none, one filing
