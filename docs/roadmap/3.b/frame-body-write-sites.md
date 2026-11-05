(§3.b frame-body-write-sites · OPEN · in Constants / parameters) (call sites of a body-writing frame function that cannot carry the call):
* symptom: call sites of a body-writing frame function that cannot carry `Terminator::Call` are E3009: CA rhs (no statement), a nested frame call (`frame_fn_lowering`), `pk::gw()` (`inline_fn.rs` reserves late), a class method, an outward / generate hier path (`hier_body_write_callee`), Rule A on a hier actual (`stmt_never_writes_ident`)
* per site
* oracles: 2 oracles
* status: OPEN
