(§5.2 Q-STRNUL · rank ① S) (strings keep interior NUL bytes that IEEE removes):
* slot: before 26
* a string holds no NUL (IEEE 1800-2017 §6.16, ieee1800-2017.txt:4470–4471; a literal's `\0` is removed, :4493–4495; `$sformat` / `$swrite` / `$sformatf` assign their result by the string-literal rules, §21.3.3 :29185–29186): vita keeps interior and trailing NULs and strips leading ones (`sim-engine/src/value.rs:941` `from_str_bytes`, `:923` `to_str_bytes`) — `$sformatf("%c%c%c%c", 8'h00, 8'h01, 8'h00, 8'h00)` is `01 00 00` len 3 (IEEE `01` len 1; iverilog 13.0 = sv2v cut at the first NUL, len 0; verilator 5.052 keeps every byte, `00 01 00 00` len 4)
* `$sformatf("%c%c%c", 8'h41, 8'h00, 8'h42)` `41 00 42` (IEEE `41 42`; iverilog = sv2v `41`; verilator `41 00 42`)
* `$swrite` / `$sformat` of `"%c%c%c", 8'h00, 8'h41, 8'h00` `41 00` (IEEE `41`; iverilog = sv2v len 0; verilator `00 41 00`).
* Consumers: `%u` / `%z` into `$sformatf`, a nested `$sformatf`, and `$sformat` / `$swrite` into a string write nothing (`$sformatf("%u", 8'h41)` len 0, IEEE `41` len 1)
* row 26's licensed `%u` / `%z` node
* source: P-PCTU plan (W/ppctu/PLAN.txt § 13 Q1, PLAN2.txt § 13 Q1; cells W/ppctu/plan nul, ss, ss2, ss3, ssl, lit, litv, rf3)
