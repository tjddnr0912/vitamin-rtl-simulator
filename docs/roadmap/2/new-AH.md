(§2 🆕 AH · OPEN · in Start-order rows) (an instance array binds a child's header at the parent's scope):
* cause: the instance-array prepass (`instance_array.rs:18` `elaborate_instance_array`, :91–144) binds a child's header parameters and port ranges at the parent's prefix with the parent's `const_func_table` and imports: a child default or port range calling `f` binds the parent's `f` (x_ia_hdr `top.u[1] p=5, u[0] p=5`, both oracles `u[0] p=5, u[1] p=a`; x_ia_port; x_t11 `p=a,p=a`, oracles `u[0] p=a, u[1] p=5`; §4.5.587's d01, d02), a child header `import q::*` is not applied (ia_pkg), and a body localparam a port range names is unbound (ia_lp `p=5,p=5`, verilator `u[1] p=a`, iverilog refuses)
* fix: run the prepass with the child's routine tables and header imports, the mirror of the `wire_ports` swap (`instance.rs:1075–1097`)
* a fold it opens from loud to a value obeys §5.2's 4-state rule (§2 🆕 AE; l22 over `fx(2)`: PRE E3009, both oracles `P=xxxx`)
* the element walk into its parent is PROBE_CATALOG §4.5.568's
* oracles: 2 oracles
* status: OPEN (§5.2 row 1)
