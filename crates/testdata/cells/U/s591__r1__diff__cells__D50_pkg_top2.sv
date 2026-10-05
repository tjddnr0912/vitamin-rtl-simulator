package p; typedef enum { FALSE, TRUE } bool_t; endpackage
package q; typedef enum { ORIGINAL, FALSE } teeth_t; endpackage
package pc;
  import p::*;
  import q::teeth_t, q::ORIGINAL, q::FALSE;
  localparam teeth_t T = FALSE;
  localparam int K = FALSE;
endpackage
module top;
  initial #1 $display("d50 T=%0d K=%0d", pc::T, pc::K);
  initial #100 $finish;
endmodule
