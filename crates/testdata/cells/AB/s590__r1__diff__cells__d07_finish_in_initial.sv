module top;
  logic [3:0] a;
  function automatic logic [3:0] f(input logic [3:0] x);
    $display("f x=%0d t=%0t", x, $time);
    f = x;
  endfunction
  wire [3:0] y = f(a);
  initial begin a = 1; $display("init"); $finish; end
  initial #5 $finish;
endmodule
