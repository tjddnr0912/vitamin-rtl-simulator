`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [8:1] AL [0:1] = '{8'hFC, 8'h02};
  logic [((X | AL[1]) == 8'hFE) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #20 $finish;
endmodule
