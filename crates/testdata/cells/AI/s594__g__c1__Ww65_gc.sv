`timescale 1ns/1ns
module t;
  localparam logic signed [64:0] X = -4;
  localparam int N = 2;
  case (X + {N{1'b0}}) 65'h1_FFFF_FFFF_FFFF_FFFC: begin : gk initial #1 $display("GC=item"); end default: begin : gd initial #1 $display("GC=def"); end endcase
  initial #5 $finish;
endmodule
