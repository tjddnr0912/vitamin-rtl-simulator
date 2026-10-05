package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module c;
  import pa::*;
  localparam int A = P;
  import pb::*;
  initial #1 $display("anw %m A=%0d P=%0d", A, P);
endmodule
module top; c u [1:0] (); initial #100 $finish; endmodule
