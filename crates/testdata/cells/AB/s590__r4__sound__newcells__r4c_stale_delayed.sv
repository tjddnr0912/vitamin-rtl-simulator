module top;
  logic a, b;
  wire p, w1, d;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic h(input logic x); integer r; r = $random; $display("h t=%0t x=%b w1=%b", $time, x, w1); return x & w1; endfunction
  assign #1 d = h(b);
  assign p = f(a);
  assign w1 = f(p);
  always @(d) $display("D t=%0t d=%b", $time, d);
  initial begin a = 1'b0; b = 1'b1; end
  final $display("final d=%b", d);
  initial #3 $finish;
endmodule
