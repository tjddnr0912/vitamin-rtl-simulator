`timescale 1ns/1ns
module t;
  function automatic int f2(input int a); return a; endfunction
  logic [15:0] v = 16'hABCD;
  initial #1 $display("pw=%h", v[0 +: ({f2(2){1'b0}} + 2'd3)]);
  initial #40 $finish;
endmodule
