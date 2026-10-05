package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
package pc;
  import pa::*;
  import pb::P;
  localparam int Z = P;
endpackage
module top;
  initial #1 $display("ppen Z=%0d", pc::Z);
  initial #100 $finish;
endmodule
