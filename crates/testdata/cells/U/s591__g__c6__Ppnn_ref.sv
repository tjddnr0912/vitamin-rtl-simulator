package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
package pc;
  import pa::*;
  import pb::*;
  localparam int Z = P;
endpackage
module top;
  initial #1 $display("ppnn Z=%0d", pc::Z);
  initial #100 $finish;
endmodule
