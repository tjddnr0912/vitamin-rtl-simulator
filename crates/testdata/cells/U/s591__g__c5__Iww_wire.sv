package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0007; endpackage
module top;
  import pa::*;
  import pb::*;
  wire [64:0] w = P;
  initial #1 $display("ww w=%0d", w);
  initial #100 $finish;
endmodule
