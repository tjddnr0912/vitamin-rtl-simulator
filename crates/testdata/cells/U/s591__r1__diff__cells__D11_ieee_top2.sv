package p; typedef enum { FALSE, TRUE } bool_t; endpackage
package q; typedef enum { ORIGINAL, FALSE } teeth_t; endpackage
module top;
  import p::*;
  import q::teeth_t, q::ORIGINAL, q::FALSE;
  teeth_t myteeth;
  initial begin myteeth = FALSE; #1 $display("d11 myteeth=%0d F=%0d", myteeth, FALSE); end
  initial #100 $finish;
endmodule
