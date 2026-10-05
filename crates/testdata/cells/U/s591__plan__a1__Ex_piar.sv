module top;
  typedef enum {E0, E1} e_t;
  leaf la[E1:0] ();
  initial #100 $finish;
endmodule
module leaf ();
  initial #3 $display("leaf %m");
endmodule
