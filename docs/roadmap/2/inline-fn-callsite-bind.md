(§2 inline-fn-callsite-bind · OPEN · in Scoping / imports / block-locals) (an inlined function body binds unbound names where the call is):
* An inlined function body binds every name that reaches the fold unbound where the CALL is (`inline_fn.rs` `reduce_function_body`: a formal is bound for the whole body, a local only from its first assignment, block-locals share one flat name space, and every declaration range folds in the caller's window), so a generate block, a caller's named block or the module answers instead of the declaring scope, whole-write lane, PRE and §4.5.615 alike: a free name (c49 `h = K` under `g: localparam K = 3`: `z=0011`, iverilog, sv2v → iverilog and verilator `z=0001`)
* a local read before its first assignment (q3c_twin `f = {a, t}` with a module `wire t`: `y=01101010`, iverilog and sv2v → iverilog `0110xxxx`, verilator `01100000`)
* a block-local in a sibling block (q3q_twin: `y=01100110`, iverilog and sv2v → iverilog `xxxx0110`, verilator `00000110`)
* a block-local shadowing a formal (q3r_twin: `y=01100110`, iverilog and sv2v → iverilog `0110xxxx`, verilator `01100000`)
* a block-local name read after its block (q3t_twin: `y=01100110`, all three `01101001`)
* a local's range naming a module constant called under a generate block that redeclares it (q3v_twin: `w=ff`, all three `w=0f`) and the return range likewise (c53: `z=00000011`, all three `00001011`)
* witness: and an imported package function's part-select read bound folds the importing module's constant (c48: `a[K+1:0]` `z=1111`, all three `0111`).
* evidence: §4.5.615's select-write merge does not widen it: a select-writing body is refused unless every name it or its declarations resolve is declared by the body and bound before it is read (`inline_ret_select.rs` `body_binding_hazard`, the literal return-range rule; row-25 lens r1 B1/B2)
* oracles: 3 oracles
* status: OPEN
