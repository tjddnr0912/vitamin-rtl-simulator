module top;
  logic [3:0] a;
  function automatic logic [3:0] f(input logic [3:0] x);
    $display("f x=%0d t=%0t", x, $time);
    f = x + 1;
  endfunction
  wire [3:0] y = f(a);
  initial a <= 1;
  initial #1 $display("y=%0d", y);
  initial #3 $finish;
endmodule
