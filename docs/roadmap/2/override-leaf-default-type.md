(§2 override-leaf-default-type · OPEN · in Index sealing) (an unreadable override leaf keeps the default's type):
* An unreadable override leaf (element, replication, call, `$rtoi`, prim cast, `SZ_0'(B8)`) keeps the default's type (Eovu_R `#(.P(X + {N{1'b0}}))` onto an untyped `P`: `P=-4 B=32`, iverilog, sv2v, verilator `P=252`)
* symptom: an untyped `localparam` holding a name-count replication or a whole element binds the unlimited fold the same way (Adiv_lv `X / {N{1'b1}}` `L=4294967295 B=32`, 3 oracles `L=84 B=8`; the literal twin Tardiv_lv is right): the declared-width value lane (`param_init_at_declared_width`; `ctx_width_names_are_evident`, the fill line's `declared_override_widths`) has no replication or element arm, so 🆕 AI's width fix alone gives these the right width and a wrong value (§4.5.594: 8 cells, Adiv_lv `L=255 B=8`)
* verilator (the `localparam` half 3 oracles)
* status: OPEN
