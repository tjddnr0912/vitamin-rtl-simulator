module top;
  typedef enum logic {A=0, B=1} e_t;
  function automatic logic [3:0] fd(input int a);
    e_t t;
    if (a == 1) t = B;
    if (t == 1'b0) fd = 4'd1; else fd = 4'd2;
  endfunction
  int n;
  initial begin n = 0; repeat (fd(2)) n = n + 1; $display("n=%0d", n); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
