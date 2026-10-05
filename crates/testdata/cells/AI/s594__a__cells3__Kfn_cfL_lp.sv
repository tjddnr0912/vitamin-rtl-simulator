`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  function automatic int fk(input int a); fk = ((X + 2'b00) + fl(a)) == 8'hFE; endfunction
  localparam L = fk(2);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
