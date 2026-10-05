package pa; localparam P = 3; localparam A = 1; endpackage
package pb; localparam P = 5; localparam B = 2; endpackage
package pc;
  import pa::*;
  import pb::*;
  localparam int Z = A + B;
endpackage
module top;
  initial #1 $display("ppnu Z=%0d", pc::Z);
  initial #100 $finish;
endmodule
