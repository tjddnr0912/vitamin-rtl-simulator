package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pb; import pa::P; export pa::P; localparam Q = 7; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("d04w P=%0d Q=%0d b=%0d", P, Q, $bits(P));
  initial #100 $finish;
endmodule
