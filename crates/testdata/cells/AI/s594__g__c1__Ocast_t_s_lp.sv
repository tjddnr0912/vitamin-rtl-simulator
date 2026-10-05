`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  typedef logic [7:0] u8_t;
  localparam L = ((C ? X : u8_t'(8'd2)) == 8'hFC);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
