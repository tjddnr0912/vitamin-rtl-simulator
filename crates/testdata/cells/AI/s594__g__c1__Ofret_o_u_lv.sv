`timescale 1ns/1ns
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  typedef logic [7:0] u8_t;
  function automatic u8_t fz(); return 8'd2; endfunction
  localparam L = X | fz();
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #5 $finish;
endmodule
