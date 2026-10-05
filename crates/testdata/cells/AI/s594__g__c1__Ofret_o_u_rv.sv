`timescale 1ns/1ns
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  typedef logic [7:0] u8_t;
  function automatic u8_t fz(); return 8'd2; endfunction
  initial #1 $display("RV=%0d", X | fz());
  initial #5 $finish;
endmodule
