package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
import pa::P;
import pb::P;
module top;
  initial #1 $display("cuee P=%0d", P);
  initial #100 $finish;
endmodule
