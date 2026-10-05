`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int fk(input int a); logic [3:0] r; r = 4'd0; fk = ((X + {N{r}}) == 8'hFC); endfunction
  int q;
  initial begin q = fk(2); #1 $display("RT=%0d", q); end
  initial #40 $finish;
endmodule
