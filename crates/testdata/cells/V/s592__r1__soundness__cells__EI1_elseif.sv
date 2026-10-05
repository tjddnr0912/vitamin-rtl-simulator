module top;
  localparam P = 2;
  if (P == 1) begin : a wire [3:0] w = 4'd1; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end else if (P == 2) begin : b wire [3:0] w = 4'd2; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end else if (P == 3) begin : c wire [3:0] w = 4'd3; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end else begin : d wire [7:0] w = 8'd200; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end
  initial #5 $finish;
endmodule
