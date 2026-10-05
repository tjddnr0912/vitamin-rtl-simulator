module top;
  typedef enum {E0, E1} e_t;
  siar #(.N(E1)) u5 ();
  initial #100 $finish;
endmodule
module siar #(parameter N = 0) ();
  leaf lf[N:0] ();
endmodule
module leaf ();
  initial #3 $display("leaf %m");
endmodule
