package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pc;
  import pb::*;
  localparam [64:0] A = P;
  import pa::*;
  localparam [64:0] B = P;
endpackage
module top;
  initial #1 $display("pwn A=%0d B=%0d", pc::A, pc::B);
  initial #100 $finish;
endmodule
