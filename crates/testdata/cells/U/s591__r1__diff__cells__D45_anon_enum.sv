package pk; localparam E1 = 7; endpackage
module top;
  import pk::E1;
  enum {E0, E1} v;
  initial #1 $display("d45 E1=%0d", E1);
  initial #100 $finish;
endmodule
