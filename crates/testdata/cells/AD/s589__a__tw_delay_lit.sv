module top;
  reg r = 0; wire w;
  assign #2 w = r;
  initial begin r = 1; #1 $display("t1 w=%b", w); #2 $display("t3 w=%b", w); $finish; end
  initial #50 $finish;
endmodule
