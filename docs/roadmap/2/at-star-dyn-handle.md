(§2 at-star-dyn-handle · OPEN · in Delays / events) (an in-body `@(*)` over a dynamic handle stays stale):
* An in-body `@(*)` over a dyn handle stays stale (compares the handle word)
* iverilog
* also `wait (r6.sum() with (item > 3.0))` never wakes on `#2 r6[0] = 3.5` (PRE 01cac4b3 prints nothing, hand `WT 3`; iverilog / sv2v have no `with`: hand only)
* OPEN
