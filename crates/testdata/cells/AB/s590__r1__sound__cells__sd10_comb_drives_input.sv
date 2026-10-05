module top;
  logic b, a;
  wire y;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  always_comb a = b;
  assign y = f(a);
  initial b = 1'b0;
  initial #1 $display("t1 y=%b", y);
  initial #10 $finish;
endmodule
