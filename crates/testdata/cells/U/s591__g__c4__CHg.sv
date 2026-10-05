module c #(parameter [64:0] i = 3) ();
  for (genvar i = 0; i < 2; i++) begin : g
    localparam int K = i;
    initial #1 $display("val %m i=%0d K=%0d", i, K);
  end
  initial #3 $display("post %m %0d", i);
endmodule
module top;
  c #(.i(65'h1_0000_0000_0000_0009)) u ();
  c v ();
  initial #100 $finish;
endmodule
