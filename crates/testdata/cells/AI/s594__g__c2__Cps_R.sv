`timescale 1ns/1ns
module t;
  localparam int N = 2;
  logic [15:0] v = 16'hFFFF;
  initial #1 $display("ps=%0d", $bits(v[0 +: ({N{1'b1}} + 2'd2)]));
  initial #20 $finish;
endmodule
