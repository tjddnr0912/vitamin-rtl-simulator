(§3.b timing-return · OPEN · in Subroutine / frame) (`return` or a self-`disable` inside a timing-control body in a task):
* symptom: a `return`, or a `disable` of the task itself, inside a timing control's body in a task (`#1 begin if (c) return; … end`) is E3009 (`return` outside a function/task method body; `disable target tk is not a lexically-enclosing named block`): `frames_classify.rs` `body_has_return` and `proc_builder.rs` `stmt_disables_name` have no timing-control arm, while the same body after a bare `#1;` runs
* oracles: iverilog (`return`: also sv2v → iverilog and verilator)
* status: OPEN
