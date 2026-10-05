module top;
  logic a, b;
  wire p, q0, q, y;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic g2(input logic x, input logic z); $display("g2 t=%0t p=%b q=%b", $time, x, z); return x ^ z; endfunction
  assign y = g2(p, q);
  assign q = f(q0);
  assign q0 = f(a);
  assign p = f(b);
  initial begin a = 1'b0; b = 1'b1; end
  initial #1 $display("t1 p=%b q=%b y=%b", p, q, y);
  initial #10 $finish;
endmodule
