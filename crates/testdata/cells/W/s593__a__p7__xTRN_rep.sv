`timescale 1ns/1ns
module t;
  function automatic logic signed [3:0] fxs(input int a); logic signed [3:0] t; return t; endfunction
  localparam bit C = 1;
  wire [7:0] r = {(((C ? fxs(2) - 4'sd4 : 4'sd0) ==? 4'sb1?00)+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
