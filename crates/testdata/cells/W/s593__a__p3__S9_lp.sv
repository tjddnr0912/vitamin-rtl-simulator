`timescale 1ns/1ns
module t;
  localparam int I = 12;
  localparam L = (I inside {4'b1?00, 4'b0011});
  initial $display("L=%b", L);
  initial #5 $finish;
endmodule
