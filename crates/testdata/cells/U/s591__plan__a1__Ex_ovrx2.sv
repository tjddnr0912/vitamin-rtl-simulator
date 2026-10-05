module top;
  typedef enum {E0, E1} e_t;
  subh #(.P(E1[3:0])) ub ();
  subh #(.P({E1, 1'b0})) uc ();
  subh #(.P(8'(E1))) ud ();
  initial #100 $finish;
endmodule
module subh #(parameter P = 0) ();
  initial #3 $display("ovrh %m P=%h b=%0d", P, $bits(P));
endmodule
