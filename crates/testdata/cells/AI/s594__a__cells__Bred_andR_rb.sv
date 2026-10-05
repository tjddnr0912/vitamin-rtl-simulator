`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic fr(input logic [3:0] a); fr = &{N{a}}; endfunction
  logic [fr(4'hF) + 2:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #40 $finish;
endmodule
