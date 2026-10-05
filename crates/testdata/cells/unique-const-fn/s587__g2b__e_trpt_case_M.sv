module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  logic clk = 0;
  always #1 clk = ~clk;
  initial begin repeat (f(2)) @(posedge clk); $display("t=%0t", $time); $finish; end
endmodule
