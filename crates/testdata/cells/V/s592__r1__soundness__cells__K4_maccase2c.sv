`define C2(s1, s2) case (s1) 1: begin : c1 wire [3:0] w = 4'd1; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end default: begin : c1d wire [7:0] w = 8'd200; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end endcase case (s2) 1: begin : c2 wire [3:0] w = 4'd2; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end default: begin : c2d wire [7:0] w = 8'd201; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end endcase
module top;
  `C2(1, 2)
  initial #5 $finish;
endmodule
