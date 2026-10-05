`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  function automatic int f2(input int a); return a; endfunction
  initial #1 $display("RV=%0d", C ? X : {f2(2){1'b0}});
  initial #5 $finish;
endmodule
