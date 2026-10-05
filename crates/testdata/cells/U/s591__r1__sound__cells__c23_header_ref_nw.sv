package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top import pa::*; #(parameter int X = P) ();
  import pb::*;
  initial #1 $display("hp X=%0d P=%0d", X, P);
  initial #100 $finish;
endmodule
