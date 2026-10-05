package pa; localparam P = 3; endpackage
package pb; localparam P = 3; endpackage
package pc;
  import pa::*;
  localparam int A = P;
  import pb::*;
  localparam int B = P;
endpackage
module top;
  initial #1 $display("n03 A=%0d B=%0d", pc::A, pc::B);
  initial #100 $finish;
endmodule
