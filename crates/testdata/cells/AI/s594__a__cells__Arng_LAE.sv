`timescale 1ns/1ns
module t;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  logic [(2'b00 + fl(2)):0] v;
  initial #1 $display("rb=%0d", $bits(v));
  initial #40 $finish;
endmodule
