`timescale 1ns/1ns
module t;
  localparam int N = 2;
  localparam L = 8'hFF + 8'd1 + {N{1'b0}};
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #5 $finish;
endmodule
