package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("aaq a=%0d b=%0d", pa::P, pb::P);
  initial #100 $finish;
endmodule
