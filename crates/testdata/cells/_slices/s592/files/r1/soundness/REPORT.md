# s592 r1 soundness lens REPORT
round: 1   status: completed   lens verdict: PASS (0 BLOCKING; 1 MAJOR comment; 3 MINOR)
binaries: PRE $S/s592/pre/vita md5 86a2d84a21d0329c57541e51ae9eb4d2 ; POST $S/s592/post_b/vita md5 020de73fcf94cad012aff0f7a3a7db62 (release)
cells: $S/s592/r1/soundness/cells (harness run.py = copy of g/run.py, D=cells)

## Findings
F1 MAJOR (comment) gen_decision.rs ForStep::Unknown doc: "an inner real, wide or unfoldable localparam never displaces
   the outer integer the Nets walk read, so every walk folds alike" is false for a wide localparam whose value fits i64.
   U8_initA_wide (inner `localparam [99:0] A = 100'd2` after the loop, outer A=1, `for (i = A; i < 3; ...)`):
   iv `g[1] g[2]` | sv `g[2]` | vl `g[2]` | PRE `g[2]` (later walks read inner A) | POST E3010 "`A` is used before ...".
   M12c_chain_wide65 (P = Q, inner Q 100-bit 2^64+2): PRE `g[2]` (P folds to low 64 bits) | POST E3010 fallback | iv g[1] g[2].
   Both via the Iterate arm (caught); the Unknown arm stays unreached in 15 probes. Z8a-d used a value > i64 only.
F1b (same comment) "No measured design reaches it": STALL1_step_zero (outer S=1, inner `localparam S = 0` after,
   `i = i + S`) reaches ForStep::Unknown against a record via the stall guard: iv g[0] g[1] g[2] | sv/vl error |
   PRE `g[0]` only (silent) | POST one E3010. Only the INIT source (M12) stays unreached. No test has a step forward ref.
F3 MINOR test gap: MS6 (For compare value-blind: Iterate(_) => expect.is_some()) SURVIVED --workspace 9134/9134;
   killed by F6_samecount_vals: MS6 `g[2] g[3]` rc0 (= PRE = sv/vl) | POST E3010 | iv `g[1] g[2]`.
F4 MINOR test gap: MS15 (no record for a zero-iteration Nets loop) SURVIVED --workspace 9134/9134;
   killed by F15_zero_nets_iter: MS15 `g[0] g[1]` rc0 (= PRE = sv/vl) | POST E3010 | iv NONE.
   MS3 (dedupe key without prefix) KILLED by cli::generate_decision_walks each_instance_array_element_is_refused.
F5 MINOR message: MSG1 names the first forward name `A` (inner A=1, same value) not the cause `B` (inner B=3).
F2 MINOR message: "declare it above the generate {kind}" on a legal shadowed design (outer object exists; IEEE binds it)
   changes the design's meaning if followed (U8, F6, F15, F3i, B1).

## Census answers
(1) walks: elaborate_generate_scoped callers = instance.rs:1066 Nets, 1193 VarInit, 1243 Logic, 1381 Instances (+recursion);
    one elaborate_instance call, that order, no return/break between (only `continue` at 1227/1262/1340).
    elaborate_instance callers: driver.rs:911 tops, instance.rs:1693 children, instance_array.rs:344 arrays.
    bind B1_bind_fwd: POST E3010 (covered); arrays V10/F3i covered (2 reports, one per element); interface generate = E3009 (V25).
(2) key: Span{lo,hi} into the expanded buffer (hdl-preprocess lib.rs:3). K1-K7 (macro x2, one macro -> 2 constructs,
    for/case, include x2 with different `define): PRE=POST=iv=sv=vl; staged==one-shot (K2,K5). One Elaborator per
    api call (api.rs:147). UNVERIFIED residue: staged multi-CU span spaces + escaped names containing '.' aliasing a prefix.
(3) replay/other inputs: R1, U1-U7, U8b, U13, M12b/d/f/g, V28b/V32/V29: no false mismatch; every mismatch traced to a
    parameter declared after the construct (U8, U8e, M12c, M12e, F6, F15, F3i, B1).
(4) For records only on cond Some(false) (`ended`); cap/stall/unfoldable exits record nothing; If/Case record on Some.
(5) forward function call: FN1, FN3, V29 no mismatch (gen-condition fold ignores generate-scope functions: FN2 PRE=POST
    "not a constant"); hierarchical H1-H5 PRE=POST "not a constant".
(6) M12 unreached: not refuted (U1-U8j, M12b-g + IMPL Z7/Z8/Z9).

staged==one-shot (PRE and POST): U8, M12c, B1, U8e, K2, K5, STALL1, F6, F15, F3i, MSG1 all SAME.
mutant bins: mut/bin_MS6 (63b716dd), bin_MS15 (0e41931e), bin_MS3 (c8dfe6e1) debug; logs mut/MS*.ws.log
