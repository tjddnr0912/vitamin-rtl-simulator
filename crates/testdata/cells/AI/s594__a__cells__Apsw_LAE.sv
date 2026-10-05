`timescale 1ns/1ns
module t;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  logic [15:0] v = 16'hABCD;
  initial #1 $display("pw=%h", v[0 +: (2'b00 + fl(2))]);
  initial #40 $finish;
endmodule
