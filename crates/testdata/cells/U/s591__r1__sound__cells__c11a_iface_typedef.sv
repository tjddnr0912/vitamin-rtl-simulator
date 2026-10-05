interface ifc;
  typedef enum {E0, E1} e_t;
  e_t v;
  initial #1 $display("it E1=%0d", E1);
endinterface
module top; ifc i(); initial #100 $finish; endmodule
