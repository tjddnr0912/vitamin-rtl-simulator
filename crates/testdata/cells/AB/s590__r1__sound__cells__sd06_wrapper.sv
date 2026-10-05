module top;
  logic a;
  wire y;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic g(input logic x); return f(x); endfunction
  assign y = g(a);
  initial a = 1'b0;
  initial #1 $display("t1 y=%b", y);
  initial #10 $finish;
endmodule
