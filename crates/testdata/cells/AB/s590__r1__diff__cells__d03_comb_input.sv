module top;
  logic [3:0] a, b;
  function automatic logic [3:0] f(input logic [3:0] x);
    $display("f x=%0d t=%0t", x, $time);
    f = x + 1;
  endfunction
  always_comb b = a + 1;
  wire [3:0] y = f(b);
  initial a = 1;
  initial #1 $display("y=%0d", y);
  initial #3 $finish;
endmodule
