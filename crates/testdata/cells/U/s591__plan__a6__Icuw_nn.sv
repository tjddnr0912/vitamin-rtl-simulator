package pa; localparam P = 3; endpackage
package pc; localparam P = 5; endpackage
import pa::*;
module top;
  import pc::*;
  localparam int K = P;
  initial #1 $display("cuwnn P=%0d K=%0d", P, K);
  initial #100 $finish;
endmodule
