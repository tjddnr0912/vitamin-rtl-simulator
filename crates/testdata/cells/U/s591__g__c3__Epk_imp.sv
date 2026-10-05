package pa; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
package pb;
  import pa::*;
  typedef enum {E0, E1} e_t;
  localparam int K = E1;
endpackage
module top;
  import pb::*;
  initial #1 $display("pki E1=%0d K=%0d b=%0d", E1, K, $bits(E1));
  initial #100 $finish;
endmodule
