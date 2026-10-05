`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  localparam L = $clog2((X + {N{1'b0}}) + fl(2));
  initial #1 $display("L=%0d", L);
  initial #50 $finish;
endmodule
