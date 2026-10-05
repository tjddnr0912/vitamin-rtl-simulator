module sub #(parameter P = 1) ();
endmodule
module top;
  if (u.P == 3) begin : a initial $display("@three"); end
  else begin : b initial $display("@other"); end
  sub #(.P(3)) u();
  initial #10 $finish;
endmodule
