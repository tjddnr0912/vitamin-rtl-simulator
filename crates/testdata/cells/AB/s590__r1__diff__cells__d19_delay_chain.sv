module top;
  logic [3:0] a;
  function automatic logic [3:0] f(input logic [3:0] x);
    $display("f x=%0d t=%0t", x, $time);
    f = x + 1;
  endfunction
  wire [3:0] y, v;
  assign #1 y = f(a);
  assign #1 v = y + 1;
  initial a = 1;
  initial begin #0; $display("t0 y=%b v=%b", y, v); #1 $display("t1 y=%b v=%b", y, v); #1 $display("t2 y=%b v=%b", y, v); end
  initial #4 $finish;
endmodule
