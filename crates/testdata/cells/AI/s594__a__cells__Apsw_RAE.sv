`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  logic [15:0] v = 16'hABCD;
  initial #1 $display("pw=%h", v[0 +: ({N{1'b0}} + fl(2))]);
  initial #40 $finish;
endmodule
