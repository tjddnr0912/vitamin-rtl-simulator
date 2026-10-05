`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  case (X | 8'h02) 8'hFE: begin : gk initial #1 $display("GC=item"); end default: begin : gd initial #1 $display("GC=def"); end endcase
  initial #20 $finish;
endmodule
