module top;
  logic a, b;
  wire p, w1, w2;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic h2(input logic x); integer r; r = $random; $display("h2 t=%0t x=%b", $time, x); return x & b; endfunction
  assign w2 = h2(w1);
  assign p = f(a);
  assign w1 = f(p);
  always @(w2) $display("W2 t=%0t w2=%b", $time, w2);
  initial begin a = 1'b0; b = 1'b1; end
  final $display("final w2=%b", w2);
  initial #3 $finish;
endmodule
