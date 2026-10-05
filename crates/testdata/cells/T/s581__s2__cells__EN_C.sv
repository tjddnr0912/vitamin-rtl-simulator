`timescale 1ns/1ns
module t;
  typedef enum logic [3:0] {EA = (4'b1100 == 4'b1100), EB} e_t;
  initial #1 $display("EN_C %0d %0d", EA, EB);
endmodule
