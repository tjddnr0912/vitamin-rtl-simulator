`define G2(c1, c2) if (c1) begin : a1 wire [3:0] w = 4'd1; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end else begin : a2 wire [7:0] w = 8'd200; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end if (c2) begin : b1 wire [3:0] w = 4'd2; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end else begin : b2 wire [7:0] w = 8'd201; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end
module top;
  localparam P = 1;
  `G2(P == 1, P == 2)
  initial #5 $finish;
endmodule
