module child #(parameter type T = bit [3:0]) (output int n);
  function automatic T fr(input int a);
    fr[0] = 1'b1;
    if (a == 1) fr = 4'd5;
  endfunction
  initial begin n = 0; repeat (fr(2)) n = n + 1; end
endmodule
module top;
  int n;
  child  u (.n(n));
  initial begin #1 $display("n=%0d", n); $display("done"); $finish; end
  initial #100 $finish;
endmodule
