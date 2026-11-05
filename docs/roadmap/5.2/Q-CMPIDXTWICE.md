(§5.2 Q-CMPIDXTWICE · rank ① · source row 29 r1 lens-diff DF2) (a compound assignment evaluates its lvalue's index expression twice):
* symptom: a compound assignment / `++` / `--` evaluates the lvalue's index expression twice: the parser desugars `lv op= e` to `lv = lv op e` (`hdl-parser/src/stmt.rs` `try_compound_assign`), so `b[f()] += 14'h1` with a side-effecting `f` calls it twice: h17a (`logic [3:0][15:2] b`) PRE 719e37d2 `R 00000010000000 n=2`, iverilog 13.0 = verilator 5.052 `R 00000000004000 n=1`, sv2v `n=2` by the same textual rewrite
* witness: h17b `m[f()][9:4] += 6'h1` PRE `R 0000 0000 0004 0000 n=2` = sv2v = verilator (iverilog aborts: `Assertion failed: (size_ == that.size_)`)
* witness: h17c `v[f()*14 + 2 +: 6] += 6'h1` PRE `R 00000040000000 n=2` = sv2v = verilator, iverilog `R 00000000010000 n=1`
* IEEE 1800-2017 §11.4.1 (ieee1800-2017.txt:11512–11513) "any left-hand index expression is only evaluated once"
* note: row 29 r1–r3 excluded such a statement (h05, PRE E3009)
* fix: hoist a non-inert lvalue index into a temp before the desugar
* status: silent-wrong, pre-existing
