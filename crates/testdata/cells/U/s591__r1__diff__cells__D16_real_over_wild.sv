package pa; localparam P = 3; endpackage
package pb; localparam real P = 2.5; endpackage
module top;
  import pa::*;
  import pb::P;
  initial #1 $display("d16 P=%f", P);
  initial #100 $finish;
endmodule
