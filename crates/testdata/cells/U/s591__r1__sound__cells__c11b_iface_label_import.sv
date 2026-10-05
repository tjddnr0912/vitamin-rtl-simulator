package pk; localparam E1 = 7; endpackage
interface ifc;
  import pk::E1;
  typedef enum {E0, E1} e_t;
  initial #1 $display("ie E1=%0d", E1);
endinterface
module top; ifc i(); initial #100 $finish; endmodule
