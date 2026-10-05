`timescale 1ns/1ns
package p;
  localparam logic [7:0] PA [0:1] = '{8'hFC, 8'h02};
endpackage
module t;
  localparam logic signed [7:0] X = -4;
  import p::*;
  localparam L = ((X | PA[1]) == 8'hFE);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
