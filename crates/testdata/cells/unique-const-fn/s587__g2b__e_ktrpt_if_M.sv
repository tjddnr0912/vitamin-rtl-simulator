module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic clk = 0;
  always #1 clk = ~clk;
  task automatic w;
    repeat (f(2)) @(posedge clk);
  endtask
  initial begin w(); $display("t=%0t", $time); $finish; end
endmodule
