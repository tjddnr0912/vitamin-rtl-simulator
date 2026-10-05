module top;
  logic a, b;
  wire p, w1, w2;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic h(input logic x); integer r; r = $random; $display("h t=%0t x=%b w1=%b", $time, x, w1); return x & w1; endfunction
  assign w2 = h(b);
  assign p = f(a);
  assign w1 = f(p);
  always @(w2) $display("W2 t=%0t w2=%b", $time, w2);
  initial begin a = 1'b0; b = 1'b1; end
  initial #1 $display("t1 w1=%b w2=%b", w1, w2);
  final $display("final w2=%b", w2);
  initial #10 $finish;
endmodule
