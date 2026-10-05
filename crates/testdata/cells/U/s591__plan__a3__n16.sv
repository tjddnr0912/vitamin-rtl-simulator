module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    sub #(.i(i)) u ();
  end
  initial #5 $display("n16 post i=%0d", i);
  initial #100 $finish;
endmodule
module sub #(parameter [64:0] i = 65'h1_0000_0000_0000_0005) ();
  for (genvar i = 0; i < 1; i++) begin : gg
    initial #3 $display("n16 sub-body %m i=%0d", i);
  end
  initial #4 $display("n16 sub-post %m i=%0d", i);
endmodule
