package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pc;
  import pa::*;
  typedef enum {P0, P} e_t;
  localparam int Z = P;
  localparam [64:0] ZW = P;
endpackage
module top;
  initial #1 $display("ppwl Z=%0d ZW=%0d", pc::Z, pc::ZW);
  initial #100 $finish;
endmodule
