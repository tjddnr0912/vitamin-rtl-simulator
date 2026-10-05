module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  sub u ();
  for (genvar i = 0; i < 2; i++) begin : g
    initial #3 $display("n17 body %m i=%0d", i);
  end
  initial #5 $display("n17 post u.i=%h top.i=%0d sel=%h", u.i, top.i, u.i[15:8]);
  initial #100 $finish;
endmodule
module sub ();
  localparam [64:0] i = 65'h1_0000_0000_0000_ab09;
  for (genvar i = 0; i < 2; i++) begin : g
    initial #3 $display("n17 sub %m i=%0d", i);
  end
endmodule
