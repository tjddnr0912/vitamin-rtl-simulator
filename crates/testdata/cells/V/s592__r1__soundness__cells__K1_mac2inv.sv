`define GIF(c, t, e) if (c) begin : t wire [3:0] w = 4'd1; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end else begin : e wire [7:0] w = 8'd200; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end
module top;
  localparam P = 1;
  `GIF(P == 1, a1, a2)
  `GIF(P == 2, b1, b2)
  initial #5 $finish;
endmodule
