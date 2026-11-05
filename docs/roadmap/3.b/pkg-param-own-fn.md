(§3.b pkg-param-own-fn · OPEN · in Constants / parameters) (a package parameter calling a function of its own package):
* symptom: a package parameter whose value calls a function of the same package is E3009 `package parameter P value is not a foldable constant [in pk]` even for `return a + 1` (§4.5.587 plan probes pk_simple, pk_plain: iverilog and verilator `P=2`, `P=10 Q=7`)
* its declared range is E3009 too (pkown `parameter logic [f(2):0] P`: `a function call that does not fold to a constant is not allowed in a constant range bound`; oracle output not kept)
* cause: `elaborate/src/package.rs:674` (the package fold's call lookup, not traced)
* witness: corpus witness row-9 R26 (`package parameter DecLcStateWidth value is not a foldable constant`, six OpenTitan pages), whose imported twin is the §2 "Scoping" import line's
* oracles: 2 oracles
* status: OPEN (§5.2 row 45)
