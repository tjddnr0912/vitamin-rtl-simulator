module top;
  function automatic logic [3:0] fc(input int a);
    logic [3:0] t;
    t = 4'd0;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fc = 4'd1; else fc = 4'd2;
  endfunction
  function automatic logic [3:0] f2(input int a);
    int t;
    if (t == 0) f2 = 4'd1; else f2 = 4'd2;
  endfunction
  int n, k;
  initial begin n = 0; repeat (fc(2)) n = n + 1; k = 0; repeat (f2(2)) k = k + 1; $display("n=%0d k=%0d", n, k); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
