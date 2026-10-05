package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
module top;
  import pa::*;
  localparam Z = P;
  import pb::*;
  localparam Y = P;
  initial #1 $display("d24m Z=%0d Y=%0d", Z, Y);
  initial #100 $finish;
endmodule
