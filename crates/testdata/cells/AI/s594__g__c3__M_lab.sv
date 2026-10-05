`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  case (8'hFC) A[0]: begin : gk initial #1 $display("GC=item"); end default: begin : gd initial #1 $display("GC=def"); end endcase
  initial #20 $finish;
endmodule
