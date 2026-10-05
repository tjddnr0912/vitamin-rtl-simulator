package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pc;
  import pa::P;
  import pb::P;
  localparam [64:0] Z = P;
endpackage
module top;
  initial #1 $display("peenw Z=%0d", pc::Z);
  initial #100 $finish;
endmodule
