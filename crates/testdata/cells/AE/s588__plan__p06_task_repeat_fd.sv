module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  function automatic logic [3:0] fx1(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    t[0] = 1'b1;
    fx1 = t;
  endfunction
  int n;
  task automatic tk(output int m); m = 0; repeat (fd(2)) m = m + 1; endtask
  initial begin tk(n); $display("n=%0d", n); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
