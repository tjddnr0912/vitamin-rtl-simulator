`timescale 1ns/1ns
package p;
  localparam logic [7:0] PA [0:1] = '{8'hFC, 8'h02};
endpackage
module t;
  localparam logic signed [7:0] X = -4;
  import p::*;
  initial #1 $display("RT=%0d", ((X | PA[1]) == 8'hFE));
  initial #20 $finish;
endmodule
