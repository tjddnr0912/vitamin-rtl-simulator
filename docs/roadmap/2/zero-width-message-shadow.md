(§2 zero-width-message-shadow · OPEN · in Diagnostics / artifacts) (a zero-width refusal names the width zero where a shadow hides the name):
* A zero-width refusal names the width "zero" where a generate block's typedef, class, interface or module instance, or function shadows the width's name and the fold reads the outer `localparam int N = 0` (t.typedef_label_blk, t.class_mod_unit, t.iface_inst_blk, t.mod_inst_blk, t.fn_bare_label_blk, t.fn_bare_decl_blk; row 65 review round 3): all three oracles refuse with a type or kind error (iverilog "Type names are not valid expressions here", "Unable to bind parameter"; verilator "Data type used where a non-data type is expected", "FUNC but expected a variable", "CELL but expected a variable")
* the claim's any-kind check (`edge_gate.rs` `binding_encloses`) reads the constant tables and `symbols` only
* loud either way, the reason is wrong
* oracles: 3 oracles
* status: OPEN
