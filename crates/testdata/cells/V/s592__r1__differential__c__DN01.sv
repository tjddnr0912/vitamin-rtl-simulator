module top;
  wire [3:0] w = 4'd1;
  if (1) begin : g
    if ($bits(w) == 8) begin : a initial #1 $display("@inner"); end
    else begin : b initial #1 $display("@outer"); end
    wire [7:0] w = 8'd2;
  end
  initial #10 $finish;
endmodule
