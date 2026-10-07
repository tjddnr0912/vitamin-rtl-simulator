package p;
  typedef enum logic [2:0] { Inv = 3'h0, Zero = 3'h1, Rma = 3'h5 } tok_e;
  parameter int N = 5;
  parameter logic [N*N*3-1:0] M = { {21{Inv}}, Zero, Rma, Inv, Zero };
endpackage
module t;
  initial begin #1 $display("A m=%h e=%h", p::M, p::M[3*3 +: 3]); $finish; end
endmodule
