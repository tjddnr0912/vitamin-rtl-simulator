module top;
  logic [31:0] a;
  function automatic logic [31:0] f(input logic [31:0] x);
    $display("f x=%0d t=%0t", x, $time);
    if (x == 5) $finish;
    return x;
  endfunction
  wire [31:0] y = f(a);
  initial begin a = 5; $display("init"); end
  initial #3 $finish;
endmodule
