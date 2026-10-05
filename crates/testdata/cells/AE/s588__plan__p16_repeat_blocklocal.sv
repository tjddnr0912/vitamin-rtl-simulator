module top;
  function automatic logic [3:0] fb(input int a);
    begin : blk
      logic [3:0] t;
      if (a == 1) t = 4'd5;
      if (t == 4'd0) fb = 4'd1; else fb = 4'd2;
    end
  endfunction
  int n;
  initial begin n = 0; repeat (fb(2)) n = n + 1; $display("n=%0d", n); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
