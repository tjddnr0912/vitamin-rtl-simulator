`timescale 1ns/1ns
module t;
  function automatic int fcase(input int a); case (a) 2: fcase = 3; default: fcase = 1; endcase endfunction
  int k;
  initial begin k = 0; repeat (2'b00 + fcase(2)) k = k + 1; #1 $display("k=%0d", k); end
  initial #40 $finish;
endmodule
