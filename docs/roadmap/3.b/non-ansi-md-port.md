(§3.b non-ansi-md-port · OPEN · in Parser accept) (a non-ANSI port declared with a multi-dimensional typedef is rejected):
* symptom: non-ANSI `input T a;` with a md typedef is E2002
* a `PortDecl` packed list is an hdl-ast SchemaHash re-pin, no format_version bump (precedent `1e1788df` `NetVarDecl.integral_typedef`; §5.2 row 70 does the same for `ParamDecl`)
* witness: cells s2.pn, sx.pn (all three run them)
* oracles: iverilog
* status: OPEN
