package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("nn P=%0d", P);
  initial #100 $finish;
endmodule
