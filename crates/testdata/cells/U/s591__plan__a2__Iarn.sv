package pa; localparam real P = 2.5; endpackage
package pb; localparam P = 3; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("arn P=%0d", P);
  initial #100 $finish;
endmodule
