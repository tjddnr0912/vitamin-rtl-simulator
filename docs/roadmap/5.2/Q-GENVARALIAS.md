(§5.2 Q-GENVARALIAS · rank ① · source row 29 r1 lens-sound F1 / F2) (a genvar reusing a variable's or an imported name is accepted):
* symptom: a genvar declared with the name of a variable or of a wildcard-imported name is accepted and the name resolves to the variable: `package p; logic [1:0][15:2] pa [2]; endpackage … import p::*; genvar pa; pa[1] = 14'h3fff;` (a16) prints `R fffXxxx xxxxxxx` on PRE 719e37d2, a17 `logic [1:0][15:2] pb; genvar pb; pb[1] = 14'h3fff;` `R fffXxxx`, where iverilog 13.0 reports `Could not find variable` and verilator 5.052 `Genvar 'pa' used outside generate for loop (IEEE 1800-2023 27.4)`
* IEEE 1800-2017 §27.4 (ieee1800-2017.txt:36449)
* note: row 29 r1 reached it through a static array on the packed chain (a4 `pa[1][9:4]`, PRE E3009) and r2 declined it
* fix: refuse the second declaration
* status: silent on invalid code, pre-existing
