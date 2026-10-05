`timescale 1ns/1ns
module t;
  function automatic logic fr(input logic [3:0] a); fr = |{2{a}}; endfunction
  localparam L = fr(4'b0010);
  initial #1 $display("L=%b", L);
  initial #40 $finish;
endmodule
