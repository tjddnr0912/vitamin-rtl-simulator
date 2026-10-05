module top;
  typedef logic [3:0] T;
  if (1) begin : g
    if ($bits(T) == 8) begin : a initial #1 $display("@inner"); end
    else begin : b initial #1 $display("@outer"); end
    typedef logic [7:0] T;
  end
  initial #10 $finish;
endmodule
