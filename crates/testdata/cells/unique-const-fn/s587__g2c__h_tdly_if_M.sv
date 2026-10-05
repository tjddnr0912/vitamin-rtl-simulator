`timescale 1ns/1ns
module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  initial begin #(f(2) * 1ns) $display("t=%0t", $time); $finish; end
endmodule
