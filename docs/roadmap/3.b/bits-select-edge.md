(§3.b bits-select-edge · OPEN · in Constant edges (§4.5.601)) (`$bits` of a select inside a part-select bound does not fold):
* symptom: a `$bits` of a select inside a part-select bound or an indexed width (`dd[2*$bits(addr[3])-1:0]`) is E3009 (`the width of this part-select does not fold to a constant: it reads addr, a variable`): the edge decision's own fold does not take the select's width, which §4.5.605's constant domain answers (iverilog `WIDTH=2`, and 21 override cells of its census)
* status: OPEN
