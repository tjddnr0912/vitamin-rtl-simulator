module top;
  typedef enum bit [3:0] {A=1, B=2, C=5} e_t;
  function automatic e_t fr(input int a);
    fr[0] = 1'b1;
    if (a == 1) fr = 4'd5;
  endfunction
  int n;
  initial begin n = 0; repeat (fr(2)) n = n + 1; $display("n=%0d", n); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
