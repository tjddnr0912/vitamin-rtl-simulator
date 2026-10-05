package pa; logic [7:0] W = 8'd5; endpackage
package pb; localparam [64:0] W = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  import pb::*;
  initial #1 $display("w W=%0d", W);
  initial #100 $finish;
endmodule
