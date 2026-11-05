(§2 md-class-property-select · OPEN · in Class fields) (a packed multi-dimensional class property select reads the flat bit):
* A packed multi-dimensional class property reads an element select as the flat bit: `p[1]` on a class `logic [1:0][3:0] p = 8'hA5` is `0` in a method (n.md_prop_ctl; verilator `a`; the module-scope twin is right)
* and a packed array of struct property's member select in a method is `xX` (q5.cls_parr_own `s[1].a`; verilator `16`)
* PRE = POST (§4.5.610 review)
* verilator
* OPEN
