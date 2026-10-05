module top;
  typedef enum {E0, E1} e_t;
  sub #(.P(E1 + 1)) ua ();
  sub #(.P(E1 - 2)) ue ();
  initial #100 $finish;
endmodule
module sub #(parameter P = 0) ();
  initial #3 $display("ovr %m P=%0d b=%0d", P, $bits(P));
endmodule
