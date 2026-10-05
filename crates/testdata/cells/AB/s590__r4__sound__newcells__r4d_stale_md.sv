module top;
  logic a, b, c;
  wire p, w1, n;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic h(input logic x); integer r; r = $random; $display("h t=%0t x=%b w1=%b", $time, x, w1); return x & w1; endfunction
  assign n = h(b);
  assign n = c;
  assign p = f(a);
  assign w1 = f(p);
  always @(n) $display("N t=%0t n=%b", $time, n);
  initial begin a = 1'b0; b = 1'b1; c = 1'bz; end
  final $display("final n=%b", n);
  initial #3 $finish;
endmodule
