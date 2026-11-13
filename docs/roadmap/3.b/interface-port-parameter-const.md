(§3.b interface-port-parameter-const · OPEN · in Constants / parameters) (an interface parameter read through an interface port in a constant):
* symptom: LOUD: an interface's parameter or localparam read through an interface port (`b.W`, `b.L`), as a value or as a range bound, is E3010 / E3009
* witness: a13_iface_param `interface bus #(parameter int W = 4); localparam int L = W * 2; logic [W-1:0] d; endinterface`, `module c(bus b);` printing `$bits(b.d), b.L, b.W`, `bus #(.W(6)) bb ();`: PRE = POST rc 1 ``undeclared hierarchical name `b.L` (no such cross-instance net or parameter)`` and the same for `b.W`
* witness: a30_iface_port_lp_range `logic [b.L-1:0] y;` (`L = W + 2`, `.W(9)`): PRE = POST rc 1 ``a hierarchical reference (`b.L`) is not allowed in a constant range bound [in top.u]``
* oracles: sv2v → iverilog and verilator `A bd=6 L=12 W=6` (a13), `A by=11` (a30); iverilog alone `t.sv:2: syntax error` / `t.sv:2: Errors in port declarations.` (W/batch4/lens-diff/oracle/a13_iface_param, a30_iface_port_lp_range)
* note: batch 4 lens-diff (W/batch4/RULINGS-lens-diff-r1.txt), PRE = POST, not batch 4's
* status: OPEN
