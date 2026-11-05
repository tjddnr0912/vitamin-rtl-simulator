(§3.b md-packed-pattern · OPEN · in Constants / parameters) (an assignment pattern into a multi-dimensional packed array is refused):
* symptom: an assignment pattern into a packed multi-dimensional array is E3009 (`an assignment pattern '{…} is supported only as the whole right-hand side of an unpacked array assignment`): a keyword md variable (x.k.pat `logic [1:0][3:0] v = '{4'h5, 4'hA}`: all three `5a`), struct elements (rv.pat_var: sv2v and verilator `v=03d`), and, once §5.2 row 70 opens the parameter lane, OpenTitan's `racl_range_t [0:0]` localparams (`spi_host_window.sv:37`, `spi_device.sv:88 / 96`)
* the `'{…}` desugar also decides struct members versus vector elements through name-keyed maps that answer for the last writer (x.cat.c571alias `w=42`, x.cat.c571formal `g=42`, all three `12`)
* `md-param-pattern-residue` is its parameter-lane sibling
* oracles: 2 oracles
* status: OPEN (§5.2 row 69)
