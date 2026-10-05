`timescale 1ns/1ns
module t;
  localparam int N = 2;
  case (8'hFF + 8'd1 + {N{1'b0}}) 8'h00: begin : gk initial #1 $display("GC=item"); end default: begin : gd initial #1 $display("GC=def"); end endcase
  initial #5 $finish;
endmodule
