`timescale 1ns/1ns
module t;
  typedef enum logic [3:0] {EA = ((4'd15 + 4'd1) inside {5'b0?000}), EB} e_t;
  initial #1 $display("EN_N %0d %0d", EA, EB);
endmodule
