module m #(parameter int N = 0) (); initial $display("@inst %m N=%0d", N); endmodule
package p1; localparam A = (4'b1100 ==? 4'b1?00); localparam B = (8'hA5 inside {8'b1010_????}) + 1; endpackage
package p2; import p1::*; localparam C = A + B + 1; localparam D = p1::A ? 8'hAA : 8'h55; endpackage
module top;
  localparam [3:0] A4 = 4'b1100;
  m u [(A4 ==? 4'b1?00) + 1 : 0] ();
  m #(.N((A4 ==? 4'b1?00) + 10)) u2 ();
  initial $display("@pk C=%0d D=%h", p2::C, p2::D);
  localparam L = (A4 ==? 4'b1?00) ? 8'hAA : 8'h55;
  localparam int LI = (A4 !=? 4'b1?00) - 1;
  initial $display("@L %h LI=%0d rep=%b bits=%0d", L, LI, {(A4 ==? 4'b1?00) + 2 {1'b1}}, $bits({8{A4 ==? 4'b1?00}}));
  for (genvar i = 0; !(i ==? 4'b1???); i++) begin : gf initial $display("@gf %0d", i); end
  for (genvar i = 0; i < 8; i++) begin : gi if (i ==? 3'b1?1) begin : y initial $display("@gi %0d", i); end end
  case (A4 ==? 4'b1?00) 1'b1: begin : c1 initial $display("@gc 1"); end default: begin : c0 initial $display("@gc def"); end endcase
endmodule
