package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top import pa::*; (output logic [P-1:0] y);
  import pb::*;
  initial #1 $display("r10 b=%0d P=%0d", $bits(y), P);
  initial #100 $finish;
endmodule
