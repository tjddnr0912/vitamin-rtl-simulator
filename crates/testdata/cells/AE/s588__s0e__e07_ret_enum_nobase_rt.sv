module top;
  int a2;
  typedef enum {A=1, B=2, C=5} e_t;
  function automatic e_t fr(input int a);
    if (a == 1) fr = C;
    if (fr == 4'd0) fr = B;
  endfunction
  int n;
  initial begin a2 = 2; n = 0; repeat (fr(a2)) n = n + 1; $display("n=%0d", n); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
