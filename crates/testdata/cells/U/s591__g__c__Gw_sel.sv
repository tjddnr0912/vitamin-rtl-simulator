module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    initial #1 $display("sel %m s0=%b s1=%b", i[0], i[1]);
  end
  initial #5 $display("post %0d", i);
  initial #100 $finish;
endmodule
