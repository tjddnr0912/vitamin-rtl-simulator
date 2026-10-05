module top;
  function automatic logic [3:0] fm(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    fm = (t & 4'b0000) + 4'd2;
  endfunction
  int n;
  initial begin n = 0; repeat (fm(2)) n = n + 1; $display("n=%0d", n); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
