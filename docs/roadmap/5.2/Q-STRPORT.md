(§5.2 Q-STRPORT · rank ① · source Q-XNONINT plan O6 (probes pd_pstr, pd_pstr2)) (a `string` input port runs silently wrong where the manual says it is refused):
* symptom: a `string` input port is silent-wrong where the manual says a string port stays loud (006_limitations "Strings \| a string as a port"): `module ps(input string si); always @(si) $display("[%s]", si); endmodule` bound `.si(v)` with v = "PP" then "QQ" prints `[ ]` then `[]` (also through `.si(c ? v : s2)`), verilator 5.052 `[PP]` / `[QQ]`
* oracles: iverilog 13.0 and sv2v → iverilog refuse `Net 'si' can not be of type 'string'` (IEEE 1800-2017 §23.2.2.3, ieee1800-2017.txt:32217-32219: an input port with no port kind is a net, and a net cannot be a string)
* symptom: the legal `input var string si` is E2002 in vita (`expected ')', found keyword 'string'`), iverilog `sorry: Port si of module ps with type string is not supported`, sv2v drops `var` (then iverilog refuses the net), verilator `[PP] 2` / `[QQ] 2`
* evidence: PRE 01cac4b3 (crates = 807a280b), native, vm and interp
* fix: refuse `input string` as documented (iverilog's line), then decide `var string` ports (support or a named refusal)
* fix: census the port-kind defaulting first
* symptom: second port class, an ANSI port's default value: `input logic [5:0] pol = 6'd8` left unconnected is W3056 and `A a=x` where iverilog, sv2v → iverilog and verilator give `A a=1` (IEEE 1800-2017 §23.2.2.4; W/row41/design/c7/z_pdef.hand.sv, c6/al_def)
* note: row 41 refuses the `=` after an alias port so the class gains no opening
* note: F if the fix changes port binding — set at grounding
