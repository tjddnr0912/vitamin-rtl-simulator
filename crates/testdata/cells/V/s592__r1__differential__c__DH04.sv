module sub ();
  if (top.P == 2) begin : a initial $display("@two"); end
  else begin : b initial $display("@other"); end
endmodule
module top;
  localparam integer P = 2;
  sub u();
  initial #10 $finish;
endmodule
