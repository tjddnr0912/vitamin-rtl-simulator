package pk; localparam string E1 = "AB"; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  sub #(.P(E1)) u ();
  sub2 #(.N(E1)) u2 (.a('0));
  initial #100 $finish;
endmodule

module sub #(parameter P = 0) ();
  initial #3 $display("ovr %m P=%0d b=%0d", P, $bits(P));
endmodule
module sub2 #(parameter N = 0) (input logic [N:0] a);
  initial #3 $display("port %m b=%0d", $bits(a));
endmodule
