`timescale 1ns/1ns
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  function automatic int f2(input int a); return a; endfunction
  localparam L = ((X + {f2(2){1'b0}}) == 8'hFC);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
