`timescale 1ns/1ns
module t;
  function automatic int f2(input int a); return a; endfunction
  logic [({f2(2){1'b0}} + 2'd3):0] v;
  initial #1 $display("rb=%0d", $bits(v));
  initial #40 $finish;
endmodule
