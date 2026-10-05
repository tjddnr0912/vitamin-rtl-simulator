package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
module top;
  import pa::*;
  localparam int A = P;
  import pb::*;
  initial #1 $display("rn A=%0d P=%0d", A, P);
  initial #100 $finish;
endmodule
