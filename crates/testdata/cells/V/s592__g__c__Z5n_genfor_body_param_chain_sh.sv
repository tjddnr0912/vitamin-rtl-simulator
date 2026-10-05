module top;
  localparam Q = 1;
  for (genvar i = 0; i < 2; i++) begin : L
    localparam P = Q + i;
    if (P == 1) begin : g wire [3:0] w = 4'd9; initial #1 $display("@L%0d one %0d bits=%0d P=%0d", i, w, $bits(w), P); end
    else begin : g wire [7:0] w = 8'd200; initial #1 $display("@L%0d other %0d bits=%0d P=%0d", i, w, $bits(w), P); end
    localparam Q = 10;
  end
  initial #5 $finish;
endmodule
