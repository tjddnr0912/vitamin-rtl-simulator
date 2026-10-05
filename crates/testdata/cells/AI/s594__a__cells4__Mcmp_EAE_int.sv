`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  localparam int L = ((X | A[1]) == (fl(0) + 8'd254));
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
