`timescale 1ns/1ns
package q;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  localparam L = ((X + {N{1'b0}}) == 8'hFC);
  localparam V = X + {N{1'b0}};
endpackage
module t;
  initial #1 $display("L=%0d V=%0d B=%0d", q::L, q::V, $bits(q::V));
  initial #50 $finish;
endmodule
