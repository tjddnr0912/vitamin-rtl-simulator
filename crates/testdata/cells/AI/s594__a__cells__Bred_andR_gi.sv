`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic fr(input logic [3:0] a); fr = &{N{a}}; endfunction
  if (fr(4'hF)) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #40 $finish;
endmodule
