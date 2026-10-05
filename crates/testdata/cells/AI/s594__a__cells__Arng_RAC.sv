`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic int fcase(input int a); case (a) 2: fcase = 3; default: fcase = 1; endcase endfunction
  logic [({N{1'b0}} + fcase(2)):0] v;
  initial #1 $display("rb=%0d", $bits(v));
  initial #40 $finish;
endmodule
