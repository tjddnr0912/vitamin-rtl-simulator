`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic logic fk(input int a); logic [3:0] r; fk = ((X + {N{r}}) == 8'hFC); endfunction
  localparam logic L = fk(2);
  initial #1 $display("L=%b", L);
  initial #40 $finish;
endmodule
