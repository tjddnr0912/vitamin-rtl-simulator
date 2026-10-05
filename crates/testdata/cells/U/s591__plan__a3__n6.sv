module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam C = i;
    localparam [64:0] CW = i;
    initial #3 $display("n6 body %m C=%0d CW=%0d bc=%0d", C, CW, $bits(C));
  end
  initial #5 $display("n6 post i=%0d g1C=%0d g1CW=%0d g0i=%0d", i, g[1].C, g[1].CW, g[0].i);
  initial #100 $finish;
endmodule
