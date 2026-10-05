# §4.5.586 unique-overlap-note — lens DIFFERENTIAL round 1
status: COMPLETE  verdict: PASS (0 BLOCKING)  wall_s≈740
binaries: PRE=S/pre/vita 8d33349579ca773389265f7a888fd17e  POST=S/post/vita 9c9c64f700f72ccdf4410659484ff720  JIT=S/post/vita_jit 470b2c97068c96af87e4f2295aafaa74  PRODUCT=S/post/vita_product cbd9d0ed66fc183f4075646a5b04bc34
harness: run.sh (vita PRE/POST + iverilog -g2012), vl.sh (verilator 5.052 --binary --timing --assert -Wno-fatal, +verilator+error+limit+1000). outputs: v/ iv/ vl/
cells: 25 designs (b01-b16 positions/controls, a01 filelists, c01-c04 fail/order/dup, e01-e04 doc claims)

## per-question
(a) once-per-run. census: 2 production frontend calls (frontend.rs:615 one-shot, pipeline.rs:519 vcmp), neither in a loop; 1 Parser per parse (api.rs:28), no sub-parser, no warnings merge. measured: -F nested filelist 1 line (a01); same file twice on argv 1 line, rc 1=1 (a02dup); --dump-filelist 0; c04 native/interp/vm/jit/product 1 each; vcmp 1, velab 0, vrun 0. vita has no -y/-v(lib)/+libext/--diag-format.
(b) latch. only consumer of Kw::Unique* = stmt.rs:165 -> parse_unique_priority. 1 line at the `unique` token for final, fork, class method, interface always_comb, package fn (unique0), generate-for, case-generate, (* attr *), label, case inside, else-unique-if under priority case, token-pasted uni``que (call site 7:12). 0 lines for \unique , unique_x, unique0_y, comment, string, unused macro (b13; stdout = iverilog = verilator).
(c) fail designs. elab fail c01 rc 1=1, --log delta = I2021 + notes; $fatal c02 rc 1=1 (also -q); -Werror / -Werror=W2004 with W2004 rc 1=1, stdout same.
(d) stdout byte-identical PRE vs POST in every run; stderr delta = exactly 1 I2021 line + notes=0->1; .vu and .velab byte-identical (md5 3c017dea...); tty: no ANSI codes; corpus-runner digests stdout only (run.rs:166-168).
(e) measured true: iverilog sorry text; per elaborated site (b07 2 lines); -s drops uninstantiated, untaken generate dropped, uncalled function counted (e03s lines 8,17 only); iverilog rejects unique if (e02); verilator doc line verbatim; 'unique if' statement violated; once per evaluation ([1],[2]); W4031 unchanged, unique0/priority0 suppress it (e04); -Wno- mnemonic/VITA- forms; -Werror=I2021 not promoted (rc 0); explain 3 forms; 71 codes = 31E 6F 3I 31W by mnemonic prefix; first match runs in all 5 engines (c04).

## findings
N1 new, harness-format, non-blocking: I2021 printed for an elaboration-failing design (c01) while iverilog, which the code comment cites as precedent ("prints its sorry only for a design it compiles"), prints none (only "c01_elabfail.sv:10: error: Unknown module type: missing_mod"). user docs only promise silence on PARSE failure, which holds.
N2 new, harness-format, non-blocking, by design: c03 prints W2004 (9:9) before I2021 (5:8) - not source order.
P1 pre-existing real gap, non-blocking: `line ignored (b15): PRE W4031 b15_line.sv:7:12, POST I2021 b15_line.sv:7:5; iverilog/verilator renamed.sv:100.
P2 pre-existing, no-oracle: verilator runs NO branch on a unique-if violation (e02 y=0, b12 y=0) but the first item on unique case (e01 y=1); vita PRE=POST first match (y=1, y=2).
P3 pre-existing, doc wording: vita lexes priority0 as a keyword; iverilog and verilator reject "priority0 if" (e04 7:15 "unexpected if"). new docs say IEEE gives priority0 no multi-match rule; IEEE has no priority0 (A.6.6, recalled: UNVERIFIED).
P4 pre-existing: program blocks do not parse (b05, 13 E2002 lines in PRE and POST), so a unique there never prints I2021.

## Round 2 (delta: soundness F1 emit below no-design-units return; N1/F2 docs; F3 test; P3 wording)
status: STARTED. POST2=S/post2/vita expected md5 9d5134826d5c2e971c6ca7bf1e841a86
status: COMPLETE  verdict: PASS (0 BLOCKING). POST2 md5 9d5134826d5c2e971c6ca7bf1e841a86 (sep vcmp 21c9036c, velab 74421968, vrun ee826236; jit d3bdc20c; product 4abff645). harness run2.sh -> v2/
POST->POST2 on round-1 design: 0 of 29 runs differ (stdout, stderr, rc); PRE rerun = round-1 PRE; c04 native/interp/vm/jit/product unchanged; sep vcmp 1 line, velab/vrun 0, .vu/.velab = PRE bytes; -Werror and -Werror=W2004 stderr = POST.
POST->POST2 on new cells: n01 (CU function only), n04 (typedef + CU function), n05 (CU localparam + task): POST printed I2021 then "error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: no design units found in source"; POST2 stderr = PRE bytes (vita and vcmp). Every other new cell: POST2 = POST.
delta checks (all POST2): preprocess error n10 (E1001), unterminated string n11 (E1013), lex error n13 (E2002 "lex error"), ifdef-empty n07: stderr = PRE bytes; n07 -DNEVER_DEFINED 1 line; CU function + module n09 1 line at 2:3; priority0-only n12 0 lines (iverilog: syntax error, so "non-standard" holds); package-only n02 / class-only n03 / interface-only n06: I2021 then "error[VITA-E3009] E-ELAB-UNSUPPORTED: no top module to elaborate" (vcmp rc 0 + line), consistent with 007's "a design unit — module, package, class or interface".
R2-1 new instance, same root class as N1, harness-format/doc-internal, non-blocking: a02dup fails with "error[VITA-E2001] E-DUP-UNIT: module `top` declared 2 times" and POST2 prints I2021 before it. 007 says the leading digit is the stage and 2xxx = Parse, and the new 007 row says "a source that fails to parse does not print it". E2001 is emitted from elaborate/src/package.rs (pre-existing band/stage mismatch) and vcmp accepts the duplicate (rc 0, .vu written, PRE same).
R2-2 same root class as N1, non-blocking: n02/n03 iverilog "No top level modules, and no -s option." with no sorry while POST2 prints I2021; n06 iverilog compiles the interface as root (crc 0, "n06_iface_only.sv:3: vvp.tgt sorry: ...") while vita refuses it with E3009 (pre-existing real gap).
