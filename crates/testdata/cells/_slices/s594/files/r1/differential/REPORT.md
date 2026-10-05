# §4.5.594 lens DIFFERENTIAL round 1 — REPORT
status: completed. verdict PASS (product does not shake). Findings: 0 BLOCKING, 1 MAJOR (comment wording), 3 MINOR (known residue classes, PRE wrong too).
binaries: PRE s594/pre/vita fc478eb9778be05e6d4ef7afa884443e; POST s594/post_a/vita 4b3afc99cfeb6a2bc5bb94cfdeca3596; staged post_a/sep vcmp 6373c0b4… velab 3de6d224… vrun 30bc6f26…
cells: c/ 38 designed, c2/ 2 designed + 2 implementer gc twins, c3/ 3 implementer pins (Ev12, Mcmp_RAE_gi/int), c4/ 6 implementer cast cells, c5/ 8 mutations (7 = c/ count probes with `== 128`→`== 8'd128` because the 32-bit 128 widened the comparison and made them vacuous; 1 = D06 with M=2).
staged == one-shot 59/59 (R: lines, final rc, VITA codes). corpus .vu/.velab PRE vs POST 30/30 byte-identical (re-measured; firing count UNVERIFIED).
descents (OK->WRONG, LOUD->WRONG, OK->LOUD, staged split, crash): 0.
MAJOR M1: const_array.rs `const_elem_wsign` doc "the element's declared type (§7.4), whatever the index" — callee `const_array_elem_read` returns None for an unfoldable / negative / out-of-range index (its own doc + `.get(idx as usize).copied()?`). Input: `A[3]` over `logic [7:0] A [2]` → no width. Product effect none measured: D20 PRE=POST E3009.
MINOR: D14 L4 residue V; D30 R5; D21 GC generate-case (literal twin Rlit_a_s_gc wrong on PRE too; POST GI=then vs GC=def internal split).
