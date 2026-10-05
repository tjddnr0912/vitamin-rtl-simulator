`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int AI [0:1] = '{-4, 2};
  logic [((AI[0] + 32'd0) == 32'hFFFF_FFFC) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #20 $finish;
endmodule
