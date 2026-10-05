package pa; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
package pb;
  import pa::*;
  typedef enum {E0, E1} e_t;
  localparam int K = E1;
  localparam [64:0] KW = E1;
endpackage
module top;
  initial #1 $display("pk E1=%0d K=%0d KW=%0d", pb::E1, pb::K, pb::KW);
  initial #100 $finish;
endmodule
