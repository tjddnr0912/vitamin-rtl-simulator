module top;
  int a2;
  typedef enum bit [3:0] {A=1, B=2, C=5} e_t;
  function automatic logic [3:0] fd(input int a);
    e_t t;
    if (a == 1) t = C;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  int n;
  initial begin a2 = 2; n = 0; repeat (fd(a2)) n = n + 1; $display("n=%0d", n); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
