#!/usr/bin/env python3
# S ladder: (LEFT, PAT) pairs -> cells Sq<NN>.sv (==?, !=? at 4 positions) and Si<NN>.sv (inside twin)
import os
P = os.path.dirname(os.path.abspath(__file__)) + '/p'
PRE = """localparam logic [3:0] A4 = 4'b1100;
localparam logic signed [3:0] S4 = -4'sd4;
parameter UP = 4'b1100;
localparam logic [64:0] W65 = {1'b1, 64'd1};
package pk; localparam logic signed [7:0] K8 = -8'sd4; endpackage
"""
pairs = [
 ("1'sb1", "2'sb1?"), ("1'b1", "2'sb1?"),
 ("4'sb1100", "8'sb1111_1?00"), ("4'b1100", "8'sb1111_1?00"),
 ("8'b1010_1100", "4'b?100"), ("8'sb1010_1100", "4'sb?100"), ("8'sb0000_0100", "4'sb?100"),
 ("33'h1_0000_0000", "33'h1_????_???0"), ("33'h0_0000_0000", "33'h1_????_???0"),
 ("-33'sd1", "64'hFFFF_FFFF_????_FFFF"), ("-33'sd1", "64'shFFFF_FFFF_????_FFFF"),
 ("64'hFFFF_FFFF_FFFF_FFFF", "64'hF???_????_????_???F"), ("-1", "64'hF???_????_????_???F"),
 ("32'shFFFF_FFFF", "64'hF???_????_????_???F"),
 ("65'h1_0000_0000_0000_0000", "65'h1_????_????_????_????"), ("-65'sd1", "128'shFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_???F"),
 ("128'h0000_0010_0000_0000_0000_0000_0000_0000", "128'h????_??1?_????_????_????_????_????_????"),
 ("8'hFF", "'bx1"), ("64'hFFFF_FFFF_FFFF_FFFE", "'bx1"), ("65'h1_0000_0000_0000_0001", "'b?1"),
 ("8'h0F", "'bz"), ("8'hA5", "'x"), ("128'h1", "'x"), ("33'h1_0000_0000", "'b?0"),
 ("8'bxxxx_0101", "8'b????_0101"), ("8'bxxxx_0101", "8'b????_0111"),
 ("(4'sd7 + 4'sd1)", "8'sb1???_1000"), ("(8'd255 + 8'd1)", "9'b1_????_????"),
 ("(32'hFFFF_FFFF + 32'd1)", "33'h1_????_???0"), ("(64'hFFFF_FFFF_FFFF_FFFF + 64'd1)", "65'h1_????_????_????_???0"),
 ("(8'h80 << 1)", "9'b1_????_????"), ("~4'b0000", "8'b1111_????"),
 ("A4", "8'b0000_1?00"), ("S4", "8'sb1111_1?00"), ("S4", "8'b1111_1?00"), ("UP", "4'b1?00"),
 ("pk::K8", "16'shFFF?"), ("W65", "65'h1_????_????_????_???1"), ("W65", "'b?1"),
 ("(S4 >>> 1)", "8'sb1111_1?10"), ("{S4, 4'h0}", "8'b1100_????"), ("(A4 ? 4'd1 : 4'd2)", "4'b000?"),
 ("4'sb1100", "'sb?100"), ("4'sb1100", "'b?100"), ("16'hFFFF", "8'sb1???_????"), ("16'h00FF", "8'sb1???_????"),
]
for n, (l, p) in enumerate(pairs):
    q = f"""{PRE}module top;
  localparam L = ({l} ==? {p});
  localparam N = ({l} !=? {p});
  logic [({l} ==? {p}) : 0] v;
  initial $display("@L %b %b bits=%0d", L, N, $bits(v));
  initial $display("@R %b %b", ({l} ==? {p}), ({l} !=? {p}));
  if ({l} ==? {p}) begin : gi initial $display("@G 1"); end else begin : ge initial $display("@G 0"); end
endmodule
// {l} ==? {p}
"""
    i = f"""{PRE}module top;
  localparam I = ({l} inside {{{p}}});
  logic [({l} inside {{{p}}}) : 0] v;
  initial $display("@I %b bits=%0d", I, $bits(v));
  initial $display("@R %b", ({l} inside {{{p}}}));
  if ({l} inside {{{p}}}) begin : gi initial $display("@G 1"); end else begin : ge initial $display("@G 0"); end
endmodule
// {l} inside {p}
"""
    open(f'{P}/Sq{n:02d}.sv', 'w').write(q)
    open(f'{P}/Si{n:02d}.sv', 'w').write(i)
print(len(pairs))
