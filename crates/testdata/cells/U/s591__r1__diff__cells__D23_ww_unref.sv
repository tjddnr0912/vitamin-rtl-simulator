package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0003; localparam A = 1; endpackage
package pb; localparam P = 5; localparam B = 2; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("d23 A=%0d B=%0d", A, B);
  initial #100 $finish;
endmodule
