`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  function automatic int fcase(input int a); case (a) 2: fcase = 3; default: fcase = 1; endcase endfunction
  logic [((A[1] - 8'd2) + fcase(2)):0] v;
  initial #1 $display("rb=%0d", $bits(v));
  initial #40 $finish;
endmodule
