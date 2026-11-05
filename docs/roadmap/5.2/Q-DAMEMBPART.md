(§5.2 Q-DAMEMBPART · rank ② · source row 24 design review D8 (d8.bl_plainnamed, pp.blocklocal_named)) (definite assignment counts a select with a named constant bound as partial):
* definite assignment counts a select write with a NAMED constant bound as a partial write: `localparam int P = 8; initial begin : b automatic logic [11:0] h; h[P+3:4] = 8'h2d; h[3:0] = 4'h5; $display("A bl h=%h", h); … end` (every bit written before the read) is E3009 "an `automatic` block-local `h` whose per-entry lifetime differs from static (an initializer, or a read before its first write) is unsupported in a procedural block …" with the note "definite-assignment for `h` stopped here: this assignment writes only PART of it (a select)" at `h[P+3:4]`, where sv2v → iverilog `A bl h=2d5` (iverilog `sorry: Overriding the default variable lifetime is not yet supported.`)
* witness: the literal twins run `2d5` (`h[11:4] = …`, and the member spelling `h.a[7:0] = …; h.s[3:0] = …;` on an automatic `h_t h`)
* cause: `elaborate/src/da/mod.rs:805` `const_index` reads a bound only as a plain decimal literal, so `const_bit_span_write` (`:770`) answers None for `P+3`
* evidence: PRE 9d60771d, every backend
* note: the named member spelling `h.a[P-1:0]` (row 24's E2002) meets the same rule once row 24 parses it
* status: loud
