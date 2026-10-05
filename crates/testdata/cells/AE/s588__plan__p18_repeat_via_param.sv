module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  localparam int N = fd(2);
  int n;
  initial begin n = 0; repeat (N) n = n + 1; $display("n=%0d N=%0d", n, N); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
