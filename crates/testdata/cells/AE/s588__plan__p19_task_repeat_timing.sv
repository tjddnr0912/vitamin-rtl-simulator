module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  reg clk = 0; always #1 clk = ~clk;
  int n;
  task automatic tk(output int m); m = 0; repeat (fd(2)) begin @(posedge clk); m = m + 1; end endtask
  initial begin tk(n); $display("n=%0d t=%0t", n, $time); end
  initial begin #20 $display("done"); $finish; end
  initial #100 $finish;
endmodule
