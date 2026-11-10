(§2 dev-overflow-release-wraps · RECORD · in Constant domain (i64)) (a dev build overflows where release wraps to the right constant):
* symptom: three constant spellings overflow an integer in a debug build where the release build wraps to the value the oracles give; no dev binary was built, so each panic is inferred from the code (PARSE-1 premise E1, E7, E8)
* witness: E1, `cw_eval`'s plain i128 `*` (now `ctx5_binop`) at w = 64 with unsigned operands from 2^63.5 up: g_p1_mul64_i128, release `bits=7`
* witness: E7, `const_eval_cast` (`const_fn.rs:831` `:864`) `(1i64 << w) - 1` at w = 63 is `i64::MIN - 1`, release wraps to the right mask; no reaching cell (the hazard the comment of `coerce_i64_to_width` names and avoids)
* witness: E8, `typedefs.rs:477` `:479` `1i128 << w` at an explicit enum base w ≥ 128: enum_w128 `typedef enum logic [127:0] {A = 5, B} e_t;` `e_t e = B;`: release PRE 56c997be, iverilog and verilator `L e=6` (W/batch2/docs/probe/SUMMARY_r1.txt)
* note: PARSE-1 (§4.5.638) kept each verbatim (lanes L4, L14, L15; W/parse1/build/NOTES.txt D5); the fix is a value row's decision (§5.2 VAL-1's cast arm)
* status: RECORD
