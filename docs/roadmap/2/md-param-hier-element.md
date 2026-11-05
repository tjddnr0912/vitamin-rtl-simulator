(§2 md-param-hier-element · OPEN · in Ranges / bounds / selects) (a multi-dimensional packed parameter read hierarchically answers a bit):
* A multi-dimensional packed parameter read hierarchically by element answers the flat bit (`u.P[1]` of `localparam logic [1:0][3:0] P = 8'hA5`: `p1=0 p0=1`, verilator `p1=a p0=5`; the md variable twin `u.V[1]` right; sv2v `0 1 0`; iverilog sorry; rv.mdp_hier): the parser flattens an md parameter (`packed_md_params`) and rewrites only the selects it sees by name
* verilator + hand-IEEE (§7.4)
* status: OPEN (§5.2 row 70)
