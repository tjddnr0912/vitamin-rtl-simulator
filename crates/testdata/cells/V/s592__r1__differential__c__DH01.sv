module sub #(parameter P = 1) ();
endmodule
module top;
  sub #(.P(3)) u();
  if (u.P == 3) begin : a initial $display("@three"); end
  else begin : b initial $display("@other"); end
  initial #10 $finish;
endmodule
