`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -4;
  for (genvar i = 0; i < 3; i++) begin : g
    localparam int K = i;
  end
  initial $display("gk=%0d", g[X+5].K);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
