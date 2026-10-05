module top;
  logic a, b;
  wire p, w1, w2;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic h(input logic x); integer r; r = $random; $display("h t=%0t x=%b w1=%b", $time, x, w1); return x & w1; endfunction
  assign w2 = h(b);
  assign p = f(a);
  assign w1 = f(p);
  initial begin a = 1'b0; b = 1'b1; end
  final $display("final w1=%b w2=%b", w1, w2);
endmodule
