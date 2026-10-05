`timescale 1ns/1ns
module t #(parameter int NH = 2);
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  case (X + {NH{1'b0}}) 8'hFC: begin : gk initial #1 $display("GC=item"); end default: begin : gd initial #1 $display("GC=def"); end endcase
  initial #5 $finish;
endmodule
