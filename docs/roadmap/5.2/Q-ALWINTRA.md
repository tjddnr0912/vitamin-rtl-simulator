(§5.2 Q-ALWINTRA · rank ① · source main D7 W/main_d7/2026-10-29/alwdly (Q-NESTCAT design review r2 G2)) (an `always` whose only timing is an intra-assignment delay runs once):
* slot: PARKED 2026-11-02 behind Q-ALWBUDGET
* slot: the slot is kept while parked
* slot: recommended high: silent-wrong on a clock-generator idiom
* slot: Q-DLYCAT waits on it
* an `always` whose only timing is an intra-assignment delay runs once: `always clk = #5 ~clk;` with `always @(posedge clk) n++;` prints `A n=1 clk=1` at 52 with W3056 "always with neither @(...) nor in-body timing is unschedulable; lowered as an inert process" at exit 0, iverilog 13.0 `A n=5 clk=0`
* witness: `always k = #10 k + 1;` `B k=1` (iverilog `B k=5`)
* cause: `stmt_has_timing` (`elaborate/src/frames_classify.rs:11`) does not count an intra-assignment `#` / `@`
* IEEE 1800-2017 §9.4.5
* evidence: PRE 01cac4b3
* status: silent-wrong on a clock-generator idiom
* note: Q-DLYCAT waits on it
* status: PARKED 2026-11-02 after design reviews dr1 and dr2 and lens rounds r1, r2 and r3 (r3 on the r3b build), nothing landed (W/qalwintra/RULINGS-lens-r3.txt; D7 W/main_d7/2026-11-02/alw-r3, binaries PRE 75486222 POST 380c2113).
* The wire, RULINGS-r3-stop: "A later route to a zero-time pass under a wrapped body, after the walk below, is T3: PARK." r3's soundness lens B1 is that route, by a premise the expression walk never read (a literal delay in a Blocking suspends): `elaborate/src/stmt_main.rs:78-84` returns on a `vif = …` before any delay is lowered, so `always begin vif = #1 i0; g <= ~g; n = n + 1; end` (vif_min) is PRE W3056 `END 20 n=1`, POST no diagnostic, killed at 2 GB in 1 s, verilator 5.052 `END 20 n=19`, and vif_if POST `END 20 n=39` with no diagnostic, verilator 5.052 `END 20 n=19`.
* B2: a1_nba PRE W3056 `END 1 w=0`, POST no diagnostic `END 9 w=0`, iverilog 13.0 and verilator 5.052 `END 9 w=1` (§2 V3/V4, the wrap takes W3056 off it).
* B3: h64 PRE `END 1`, POST killed at 20 s, 8 MB, no fatal.
* This is the fifth instance on one root class, the walker a parallel model of the lowering (dr1 B1, r1 F-A F-B, r2 F-S2, r3 STOP 1, r3 B1), so the conjuncts that exist at 0 moves lost by the lens's regex (no virtual-interface write; no routine or function chain, 0 of 348) are not taken: the wire was written against the next instance.
* PRICE: nothing lands.
* `always clk = #5 ~clk;` keeps PRE's run-once with W3056
* record: 227 moves, 107 W>r, 69 of 117 kept do not land
* record: Q-DLYCAT stays behind it.
* Main is unchanged: loud, no new silent shape.
* Record: W/qalwintra/r3b/code.patch (decc64eb) and docs.patch (eecba757) on abb7973a, EXPECT.tsv, pins, battery and the lens cells W/qalwintra/r3b/lens-sound stay
* record: the r3 differential lens was stopped by main, nothing in it reached a verdict (W/qalwintra/r3b/lens-diff stays).
* Re-entry: behind Q-ALWBUDGET.
* Once it lands the walker need not carry the proof: an unbounded or zero-time pass is that row's fatal, whatever route reached it.
* No budget fires on a wrong value in a pass that ends, so the re-plan still names vif_if (Q-ALWRESIDUE's Q-VIFDLY) and a1_nba (§2 V3/V4)
* source: Q-ALWINTRA r3 lens-sound B1–B3 (W/qalwintra/r3b/lens-sound/REPORT.txt), W/qalwintra/RULINGS-lens-r3.txt
