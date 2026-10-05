module top;
  logic a;
  function automatic logic f(input logic x);
    $display("f x=%b t=%0t", x, $time);
    f = x;
  endfunction
  wire y = f(a);
  always @(y) $display("y=%b t=%0t", y, $time);
  initial begin a = 1; force y = 0; #3 release y; #1 $display("#4 y=%b", y); end
  initial #6 $finish;
endmodule
