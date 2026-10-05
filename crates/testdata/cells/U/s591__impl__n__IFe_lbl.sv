package pk; localparam E1 = 7; endpackage
interface ifc;
  import pk::E1;
  typedef enum {E0, E1} e_t;
  int k;
  initial begin k = E1; #1 $display("ife E1=%0d k=%0d", E1, k); end
endinterface
module top;
  ifc i();
  initial #100 $finish;
endmodule
