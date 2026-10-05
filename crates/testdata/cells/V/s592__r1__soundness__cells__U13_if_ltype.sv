module top;
  localparam A = 1;
  if (1) begin : b
    if (A == 1) begin : x wire [3:0] w = 4'd1; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end else begin : y wire [7:0] w = 8'd200; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end
    localparam type A = logic [7:0];
  end
  initial #5 $finish;
endmodule
