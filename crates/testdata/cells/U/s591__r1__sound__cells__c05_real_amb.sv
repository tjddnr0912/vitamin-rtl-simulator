package pa; localparam real P = 1.5; endpackage
package pb; localparam P = 5; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("ra P=%0d", P);
  initial #100 $finish;
endmodule
