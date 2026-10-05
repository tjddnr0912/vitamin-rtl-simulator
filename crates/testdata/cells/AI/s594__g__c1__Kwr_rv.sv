`timescale 1ns/1ns
module t;
  localparam int N = 2;
  initial #1 $display("RV=%0d", 8'hFF + 8'd1 + {N{1'b0}});
  initial #5 $finish;
endmodule
