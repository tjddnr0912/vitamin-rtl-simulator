interface ifc;
  typedef enum {E0, E1} e_t;
  logic [3:0] x;
  initial #1 $display("d29a E1=%0d", E1);
endinterface
module top;
  ifc u();
  initial #100 $finish;
endmodule
