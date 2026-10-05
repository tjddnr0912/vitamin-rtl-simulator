`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  function automatic int fk(input int a); fk = ((X + {N{1'b0}}) + fl(a)) == 8'hFE; endfunction
  int q;
  initial begin q = fk(2); #1 $display("RT=%0d", q); end
  initial #40 $finish;
endmodule
