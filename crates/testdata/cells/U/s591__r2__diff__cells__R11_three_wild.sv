package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pc; localparam P = 7; endpackage
module top;
  import pa::*;
  localparam int A = P;
  import pb::*;
  import pc::*;
  initial #1 $display("r11 A=%0d P=%0d", A, P);
  initial #100 $finish;
endmodule
