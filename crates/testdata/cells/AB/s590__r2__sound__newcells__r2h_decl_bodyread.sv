module top;
  logic a, b, gs;
  wire w1, w2;
  function automatic logic f1(input logic x); $display("f1 t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic f2(input logic x); gs = x; $display("f2 t=%0t x=%b w1=%b", $time, x, w1); return x; endfunction
  assign w2 = f2(b);
  assign w1 = f1(a);
  initial begin a = 1'b0; b = 1'b1; end
  initial #1 $display("t1 w1=%b w2=%b", w1, w2);
  initial #10 $finish;
endmodule
