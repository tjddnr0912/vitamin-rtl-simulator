package pa; localparam string P = "ab"; endpackage
package pb; localparam P = 5; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("sa P=%0d", P);
  initial #100 $finish;
endmodule
