`timescale 1ns/1ns
module t;
  localparam int N = 2;
  localparam logic [7:0] A8 = 8'h01;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  initial #1 $display("rep=%h", {(A8[{2{1'b0}}] + fl(2)){4'hF}});
  initial #40 $finish;
endmodule
