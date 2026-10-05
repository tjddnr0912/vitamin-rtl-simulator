module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  logic clk = 0; int x, y = 5;
  always #1 clk = ~clk;
  initial begin x = repeat (f(2)) @(posedge clk) y; $display("x=%0d t=%0t", x, $time); $finish; end
endmodule
