`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int fk(input int a); logic [7:0] r; fk = $clog2((X + {N{1'b0}}) + r); endfunction
  localparam L = fk(2);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
