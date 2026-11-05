(§5.2 Q-REFMD · rank ① · source ENG-2 r2 lens-diff N-REFMD (T06, P12)) (an initialized variable passed to a read-only `ref` is falsely multi-driven):
* symptom: a `logic g = 0;` (declaration-initialized) passed to a read-only `ref` or a `const ref` formal in an `always_comb` is a false `VITA-E3001` ("has a declaration initializer AND is written by always_comb") on a legal design (loud) where verilator runs it (`C t=0 o=0 \| C t=5 o=1 \| A end o=1`)
* cause: the multidriver census still sees the parser's `inout` copy-out of a `ref`
* status: pre-existing (PRE = POST)
* IEEE 1800-2017 §13.5.2 :14840-14841 (a `const ref` cannot alter the variable)
* fix: the census asks the read set's write collector what the call writes
