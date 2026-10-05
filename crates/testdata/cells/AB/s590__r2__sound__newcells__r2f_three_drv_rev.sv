module top;
  logic a, b;
  wire [2:0] n;
  wire [2:0] y;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic [2:0] gn(input logic [2:0] x); $display("gn t=%0t n=%b", $time, x); return x; endfunction
  assign y = gn(n);
  assign n[2] = f(n[0] ^ n[1]);
  assign n[0] = f(a);
  assign n[1] = f(b);
  initial begin a = 1'b0; b = 1'b1; end
  initial #1 $display("t1 n=%b y=%b", n, y);
  initial #10 $finish;
endmodule
