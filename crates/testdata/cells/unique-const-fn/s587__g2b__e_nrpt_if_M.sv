module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic clk = 0; int x, y = 5;
  always #1 clk = ~clk;
  initial begin x <= repeat (f(2)) @(posedge clk) y; #40 $display("x=%0d", x); $finish; end
endmodule
