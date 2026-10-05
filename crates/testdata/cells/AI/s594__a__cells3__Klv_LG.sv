`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  function automatic logic [7:0] fg(input int a); logic [7:0] r; r = 8'd0; fg = r + a; endfunction
  localparam L = X + 2'b00 + fg(2);
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #40 $finish;
endmodule
