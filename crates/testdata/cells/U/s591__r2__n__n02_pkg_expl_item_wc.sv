package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
package pc;
  import pa::P;
  localparam X = 1;
  import pb::*;
  localparam int Y = P;
endpackage
module top;
  initial #1 $display("n02 Y=%0d", pc::Y);
  initial #100 $finish;
endmodule
