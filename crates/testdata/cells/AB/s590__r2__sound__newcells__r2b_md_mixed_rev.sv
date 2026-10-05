module top;
  logic a, b;
  wire n, y;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic g(input logic x); $display("g t=%0t x=%b", $time, x); return x; endfunction
  assign y = g(n);
  assign n = f(a);
  assign n = b;
  initial begin a = 1'b1; b = 1'bz; end
  initial #1 $display("t1 n=%b y=%b", n, y);
  initial #10 $finish;
endmodule
