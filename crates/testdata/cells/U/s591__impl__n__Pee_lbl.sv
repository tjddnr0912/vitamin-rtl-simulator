package pk; localparam E1 = 7; endpackage
package pc;
  import pk::E1;
  typedef enum {E0, E1} e_t;
  localparam int Z = E1;
endpackage
module top;
  initial #1 $display("pel Z=%0d", pc::Z);
  initial #100 $finish;
endmodule
