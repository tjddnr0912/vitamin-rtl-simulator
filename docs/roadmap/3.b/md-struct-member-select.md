(§3.b md-struct-member-select · OPEN · in Parser accept) (a member select after a two-level index into a packed struct array):
* symptom: a member select after a two-level index of a packed array of structs (`a[i][k].c`, adc_ctrl `adc_ctrl_core.sv:106`) is E2002 (`expected ')', found '.'`)
* ⑤ⓔ's sibling
* cause: `struct_sel.rs` takes the element chain to the struct element before the member
* all three `A o=101`
* witness: row-9 R23
* status: OPEN (§5.2 row 38)
