`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  int k;
  initial begin k = 0; repeat ((A[1] - 8'd2) + fl(2)) k = k + 1; #1 $display("k=%0d", k); end
  initial #40 $finish;
endmodule
