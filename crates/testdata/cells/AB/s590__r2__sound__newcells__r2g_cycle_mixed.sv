module top;
  logic a, b;
  wire [2:0] c;
  wire p, y, z;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic g(input logic x); $display("g t=%0t x=%b", $time, x); return x; endfunction
  assign z = g(c[2]);
  assign y = g(p);
  assign p = f(b);
  assign c[2] = f(c[1]);
  assign c[1] = f(c[0]);
  assign c[0] = f(a);
  initial begin a = 1'b0; b = 1'b1; end
  initial #1 $display("t1 c=%b p=%b y=%b z=%b", c, p, y, z);
  initial #10 $finish;
endmodule
