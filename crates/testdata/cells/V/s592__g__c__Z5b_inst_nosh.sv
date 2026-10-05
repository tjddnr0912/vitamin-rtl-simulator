module sub #(parameter P = 0) (); initial #1 $display("@%m P=%0d", P); endmodule
module top;
  if (1) begin : gb
    sub #(.P(K)) u ();
    localparam K = 8;
  end
  initial #5 $finish;
endmodule
