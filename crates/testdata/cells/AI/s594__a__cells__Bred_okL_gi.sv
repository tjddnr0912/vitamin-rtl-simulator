`timescale 1ns/1ns
module t;
  function automatic logic fr(input logic [3:0] a); fr = |{2{a}}; endfunction
  if (fr(4'b0010)) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #40 $finish;
endmodule
