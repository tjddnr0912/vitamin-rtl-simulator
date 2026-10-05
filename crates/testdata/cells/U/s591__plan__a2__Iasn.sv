package pa; localparam string P = "xy"; endpackage
package pb; localparam P = 3; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("asn P=%0d", P);
  initial #100 $finish;
endmodule
