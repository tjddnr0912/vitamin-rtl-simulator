`timescale 1ns/1ns
module t;
  function automatic logic fr(input logic [3:0] a); fr = |{2{a}}; endfunction
  logic [fr(4'b0010) + 2:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #40 $finish;
endmodule
