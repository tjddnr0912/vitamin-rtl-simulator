package pa; localparam P = 3; endpackage
package pb; import pa::P; export pa::P; localparam Q = 7; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("d04 P=%0d Q=%0d", P, Q);
  initial #100 $finish;
endmodule
