package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
package pc;
  import pa::*;
  localparam Z = P;
  import pb::*;
  localparam Y = P;
endpackage
module top;
  initial #1 $display("d24 Z=%0d Y=%0d", pc::Z, pc::Y);
  initial #100 $finish;
endmodule
