package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pc;
  import pa::*;
  import pb::P;
  localparam int K = P;
  localparam [64:0] Z = P;
endpackage
module top; initial #1 $display("pw K=%0d Z=%0d", pc::K, pc::Z); initial #100 $finish; endmodule
