module top;
  function automatic logic [3:0] hm(input int x); return x; endfunction
  reg r = 0; wire w;
  assign #(hm(18)) w = r;
  initial begin r = 1; #1 $display("t1 w=%b", w); #2 $display("t3 w=%b", w); $finish; end
  initial #50 $finish;
endmodule
