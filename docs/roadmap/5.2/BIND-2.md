(§5.2 BIND-2 · rank — F · source W/program/reorder2-2026-11-09/PLAN.txt G2 BIND-2) (the constant-function frame as bound-constant records):
* slot: R4.2 · L1, after BIND-1 (owner decision O3, 2026-11-09)
* scope: structure only: every output equals PRE (values, diagnostics, exit code, `.vu` and `.velab` bytes, termination, cost), PRE's differences kept as named lanes (owner, 2026-11-09)
* fix: the constant-function frame (`env` `BTreeMap<String,i64>` + `envw` `ConstWidths` (u32, bool) + `const_unwritten`, 13 writes, `const_fn.rs` / `const_fn_width`) as one `BoundConst` per local, a second store beside BIND-1's because its lifetime differs
* note: lanes: the width-0 absorbers (`const_fn.rs:1476` `:1484` `:1674`, `inline_fold.rs:79`) and `const_range_numbered`'s `[N:0]` numbering
* note: order: `bind_const_decl` (`const_fn.rs:1455`) folds a local's range where PRE does (`RangeFacts::lazy`: a range may call a constant function)
* pins: j4r stays `A P=1` (§2 const-fn-local-unbound-width)
* note: size 4–5 files; risk medium
* note: it frees Q-CONSTSELLSB, Q-CONSTAGG and §2 const-fn-local-unbound-width (R7.1–R7.3) and 76 (its x plane, after 48)
* note: the home rows' trip wires and their honest cost are written once, in §5.2 SYS-1
* source: W/program/reorder2-2026-11-09/PLAN.txt G2 BIND-2
