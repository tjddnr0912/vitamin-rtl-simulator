`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  typedef logic [7:0] u8_t;
  function automatic u8_t fz(); return 8'd2; endfunction
  initial #1 $display("RT=%0d", ((X | fz()) == 8'hFE));
  initial #5 $finish;
endmodule
