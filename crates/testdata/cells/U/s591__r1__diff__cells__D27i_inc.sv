`include "D27inc.svh"
package pk; localparam E1 = 7; endpackage
module top;
  import pk::E1;
  e_t v = E0;
  initial #1 $display("d27i v=%0d E1=%0d", v, E1);
  initial #100 $finish;
endmodule
