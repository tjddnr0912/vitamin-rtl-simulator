module sub #(parameter P = 1) ();
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : a initial $display("@%m inner"); end
    else begin : b initial $display("@%m outer"); end
    localparam integer K = P;
  end
endmodule
module top;
  sub #(.P(2)) u1();
  sub #(.P(1)) u2();
  initial #10 $finish;
endmodule
