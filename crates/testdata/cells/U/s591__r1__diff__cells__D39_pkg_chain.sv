package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
package pc; import pa::P; localparam Z = P + 10; endpackage
package pd; import pb::P; localparam Y = P + 20; endpackage
module top;
  import pc::*;
  import pd::*;
  initial #1 $display("d39 Z=%0d Y=%0d", Z, Y);
  initial #100 $finish;
endmodule
