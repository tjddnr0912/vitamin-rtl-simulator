(§5.2 Q-UNSZXPAD · rank ① · source §4.5.569 review (differential lens F1 p32 / p34a / p36b, implementer u1 / b3)) (an unsized x / z literal is zero-extended past 32 bits):
* slot: after 48
* slot: before Q-MEMBSRC
* an unsized based literal led by `x` or `z` is read at 32 bits and zero-extended where IEEE 1800-2017 §5.7.1 (ieee1800-2017.txt:3006–3008) extends its x / z to the size of the expression that contains it: `v = 'bx;` into `logic [127:0]` prints `000000000000000000000000xxxxxxxx` where iverilog and sv2v → iverilog print all `x` (verilator, 2-state, `0`)
* `localparam logic [127:0] V = 'bx;` the same (verilator and iverilog all `x`)
* inside an operand (`v = 'bx \| 128'h0;`) and through a size cast (`{64'('bx), 64'('hz)}`: `00000000xxxxxxxx00000000zzzzzzzz`, both oracles `x…x z…z`) likewise
* in a constant fold the padded value is a KNOWN value an operator routes on: `logic [(('hx0 === 64'hxxxxxxxx_xxxxxxx0) ? 4'd1 : 4'bx):0] d;` PRE n=1, iverilog 13.0 = sv2v → iverilog = verilator 5.052 n=2 (Q-DECLSELBOUND plan 3 u01; u02 u05 u06 u08 u10)
* `elaborate/src/literal.rs` `parse_int_literal` (an unsized literal is 32 bits) and the context extension, which zero-fills an unsigned operand (`const_wide.rs` `fold_region`, the lowering, every run-time lane)
* a packed-struct container it pads feeds Q-MEMBSRC (`localparam hw_t P = 'bx;` over 128 bits: under Q-MEMBSRC's PLAN3 F `P.a[~N]` reads 0 and `[P.a[~N] + 1:0]` is n=2 where IEEE has x and an illegal bound; PRE E3009; W/row24/membsrc/plan4/cells/pva/a28_unszx)
* moved from PROBE_CATALOG (§4.5.569 review, CLASS) into Q-DECLSELBOUND's fix path
* PRE 9d60771d
* silent-wrong, pre-existing
* source: Q-DECLSELBOUND plan 3 T2
