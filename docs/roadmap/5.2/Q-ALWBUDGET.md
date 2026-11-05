(§5.2 Q-ALWBUDGET · rank — · source Q-ALWINTRA r3 lens-sound B1, B3 (W/qalwintra/r3b/lens-sound/REPORT.txt)) (an engine budget: a pass that never suspends ends in a fatal, not OOM or a hang):
* slot: past the last numbered slot
* slot: recommended high: it holds Q-ALWINTRA and, through it, Q-DLYCAT
* slot: starts only on a track whose next C row cannot start
* slot: `native/`, `backend.rs`: flip run
* scope: an engine budget: a process pass with no suspension, past a budget, ends in a fatal instead of OOM or a hang
* no oracle needed (a guard).
* Witnesses, cells W/qalwintra/r3b/lens-sound/cells, POST = the parked Q-ALWINTRA build (md5 380c2113, not on main), PRE 719e37d2 (md5 75486222): vif_min (c2) `interface bus; logic v = 0; endinterface`, `module top; bus i0(); virtual bus vif; reg g = 0; integer n = 0; always begin vif = #1 i0; g <= ~g; n = n + 1; end initial #20 begin $display("END %0d n=%0d", $time, n); $finish; end endmodule`: POST no diagnostic, native vm interp killed at 2 GB in 0.88 / 1.25 / 2.60 s, staged 1.22 s, no fatal
* oracles: verilator 5.052 `END 20 n=19`, iverilog 13.0 `t.sv:3: syntax error` `t.sv:3: error: Invalid module item.`, sv2v `` t.sv:3:13: Parse error: missing expected `endmodule` ``
* note: the route is POST's alone, PRE W3056 `END 20 n=1`
* PRE twin vif_stmt_twin (`always begin #0; vif = #1 i0; g <= ~g; n = n + 1; end`) `fatal[VITA-F4016] F-RUN-NO-CONVERGE: did not converge: delta limit (1000000) exceeded at time 0 (zero-delay loop / combinational oscillation) [at time 0]` rc 1, 51 MB, 0.13 s: its `#0` meets the delta limit, and the pass with no suspension is reached on PRE by `always begin #1 x = ~x; n = n + 1; if (n > 2) forever w <= ~w; end` with a 1 Mbit `w` (W/qalwintra/r2/lens-sound/cells/a7/t_f1), killed at 2 GB in 0.66 s, no fatal.
* h64 (b4), 65 functions `function automatic integer e0(integer v); e0 = e1(v) + e1(v+1); endfunction` … `function automatic integer e64(integer v); e64 = v; endfunction`, no loop, `always begin x = #1 ~x; n = n + 1; if (n > 2) y = e0(n); end`, `initial #100 begin $display("END %0d y=%0d q=%0d", n, y, q); $finish; end`: PRE W3056 `END 1 y=0 q=0`, POST no W3056, native vm interp killed at 20 s, staged 6 s, 8 MB flat, no fatal, time stays at 3
* the ladder ef16 0.07 s, ef20 0.64 s, ef24 13 s, value right.
* PRE twin h64_twin (`always #1 begin x = ~x; …`) on PRE native vm interp killed at 20 s, 8 MB flat, no fatal
* oracles: iverilog and sv2v → iverilog killed at 8 s, flat 5 MB, no output
* oracles: verilator compile killed (`%Error: Verilator threw signal 9. Suggest trying --debug --gdbbt`) on h64 and on the twin: no oracle has a value.
* witness: tk_lcl64 (b4) reaches it through tasks, no function: `task automatic e0; begin e1; e1; end endtask` … `task automatic e64; integer i; i = 1; endtask`, `always begin x = #1 ~x; n = n + 1; if (n > 2) e0; end`: PRE W3056 `END 1 y=0 q=0`, POST native killed at 20 s, 7 MB, no fatal
* h64_twin is the PRE twin.
* Mechanism: `run_frame_call_inner` (`sim-engine/src/state/frame_eval.rs:1508`) has a depth guard and no step budget
* `max_body_steps` is charged by the process executor (`native/body.rs:731`, `backend.rs:1143`), not there.
* F4027 exists and does not fire when an NBA is scheduled: c1/vif.sv (`always begin vif = #1 i0; n = n + 1; end`) POST `` fatal[VITA-F4027] F-RUN-BODY-STEP-LIMIT: one process executed 100000000 block steps at time 0 without reaching a `#delay`, `@(…)` or `wait` — either it is an unbounded loop, or it is a long computation that needs a larger budget (`SimOpts::max_body_steps`) [at time 0] `` rc 1 at 2.59 s
* witness: vif_min and c1/vif_nba, the same with `g <= ~g;`, reach the 2 GB cap first
* past the cap UNVERIFIED (§5.b NBA-GROWTH: F4027 at 4.7 GB under a 6 GB cap on one NBA, none before 6 GB on four).
* So a step count alone leaves vif_min.
* `task automatic r; fork x = ~x; join r; endtask always r;` (s40, W/qalwintra/r2/lens-diff/cells/d5) has no fatal on native (PRE and POST killed at 8 s, 21 MB) where POST vm and interp end in F4027 rc 1 in 0.14 / 0.12 s, inside the one pass an inert `always` runs
* PRE vm and interp not run, root not found, no oracle has a value.
* Measured before this row, Q-ALWINTRA's replan rejected each (W/qalwintra/replan/REPLAN.txt:35–37, xe/SUMMARY.txt): a byte budget, at least 37 charge sites, no funnel, MEM-GUARD BLOCKED, no reach on a flat hang
* note: a fork budget K, where no K closes the growth and spares `legal`, 100,000 `join_none` forks in one step that end rc 0 (F4027 at K <= 65536; big200 OOM at every K)
* a zero-time pass budget on one `always`, NOT prototyped, big200 (one pass, 200 MB) against it.
* The design starts there.
* Exit: vif_min, h64, tk_lcl64 and s40 end in a fatal on every lane, `legal` stays rc 0, NBA-GROWTH closes with it or says why not
* Q-ALWINTRA then re-plans with no proof in its walker.
* UNVERIFIED: tk_lcl64 on vm, interp and staged
* note: the budget's value
* note: natural saturation at 2^32 passes
* source: W/qalwintra/RULINGS-lens-r3.txt
