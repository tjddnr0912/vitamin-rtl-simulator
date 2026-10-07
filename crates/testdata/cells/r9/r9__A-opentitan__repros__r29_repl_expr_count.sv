package p;
  typedef enum logic [2:0] { Inv = 3'h0, Zero = 3'h1, Rma = 3'h5 } tok_e;
  `define ROW(idx) {2{Zero}}, {(3-idx){Inv, Zero}}, {(2*idx+1){Inv}}
  parameter logic [2*9*3-1:0] M = { `ROW(1), `ROW(2) };
endpackage
module t;
  initial begin #1 $display("A m=%h", p::M); $finish; end
endmodule
