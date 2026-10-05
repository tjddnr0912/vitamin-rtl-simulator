`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  typedef logic [7:0] u8_t;
  case (C ? X : u8_t'(8'd2)) 8'hFC: begin : gk initial #1 $display("GC=item"); end default: begin : gd initial #1 $display("GC=def"); end endcase
  initial #5 $finish;
endmodule
