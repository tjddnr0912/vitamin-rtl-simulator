`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic int fcase(input int a); case (a) 2: fcase = 3; default: fcase = 1; endcase endfunction
  logic [15:0] v = 16'hABCD;
  initial #1 $display("pw=%h", v[0 +: ({N{1'b0}} + fcase(2))]);
  initial #40 $finish;
endmodule
