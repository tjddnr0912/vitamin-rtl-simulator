`timescale 1ns/1ns
module t;
  localparam int N = 2;
  localparam L = ((8'hFF + 8'd1 + {N{1'b0}}) > 8'd100);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
