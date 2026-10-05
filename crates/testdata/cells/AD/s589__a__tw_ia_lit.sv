module child; initial #1 $display("%m"); endmodule
module top;
  child u[1:0] ();
  initial #2 $finish;
  initial #50 $finish;
endmodule
