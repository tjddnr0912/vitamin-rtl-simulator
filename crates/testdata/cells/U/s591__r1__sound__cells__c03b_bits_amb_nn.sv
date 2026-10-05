package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("bb b=%0d", $bits(P));
  initial #100 $finish;
endmodule
