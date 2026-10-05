`timescale 1ns/1ns
module t;
  function automatic int f2(input int a); return a; endfunction
  localparam V = {f2(2){1'b1}};
  initial #1 $display("V=%%0d B=%%0d", V, $bits(V));
  initial #5 $finish;
endmodule
