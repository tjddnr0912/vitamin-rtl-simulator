module top;
  function automatic logic [3:0] fs(input int a);
    logic [3:0] t;
    t = t + 4'd1;
    fs = t;
  endfunction
  int n;
  initial begin n = 0; repeat (fs(2)) n = n + 1; $display("n=%0d", n); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
