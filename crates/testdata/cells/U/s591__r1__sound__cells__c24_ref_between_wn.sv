package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pb::*;
  localparam [64:0] A = P;
  import pa::*;
  initial #1 $display("rw A=%0d P=%0d", A, P);
  initial #100 $finish;
endmodule
