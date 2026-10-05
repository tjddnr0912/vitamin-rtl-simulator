`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic [7:0] fg(input int a); logic [7:0] r; r = 8'd0; fg = r + a; endfunction
  logic [15:0] v = 16'hABCE;
  initial #1 $display("pw=%0d", v[0 +: ({N{1'b0}} + fg(2))]);
  initial #40 $finish;
endmodule
