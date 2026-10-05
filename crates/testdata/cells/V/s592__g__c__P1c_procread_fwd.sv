module sub #(parameter P = 0) (); initial #1 $display("@%m P=%0d", P); endmodule
module top;
  localparam K = 4;
  if (1) begin : gb
    logic [7:0] v = K;
    initial #1 $display("@K=%0d v=%0d", K, v);
    sub #(.P(K)) u ();
    localparam K = 8;
  end
  initial #5 $finish;
endmodule
