`timescale 1ns/1ns
module t;
  localparam int N = 2;
  localparam logic [7:0] CA [0:1] = '{8'd0, 8'd1};
  localparam logic [7:0] A8 = 8'h01;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  int k;
  initial begin k = 0; repeat (A8[{N{1'b0}}] + fl(2)) k = k + 1; #1 $display("k=%0d", k); end
  initial #50 $finish;
endmodule
