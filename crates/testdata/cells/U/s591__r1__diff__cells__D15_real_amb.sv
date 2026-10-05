package pa; localparam real R = 1.5; endpackage
package pb; localparam real R = 2.5; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("d15 R=%f", R);
  initial #100 $finish;
endmodule
