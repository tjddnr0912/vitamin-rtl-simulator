package pa; localparam P = 3; endpackage
package pb; logic [7:0] P = 8'd5; endpackage
package pc;
  import pa::*;
  localparam X = 1;
  import pb::P;
endpackage
module top;
  initial #1 $display("n10 X=%0d", pc::X);
  initial #100 $finish;
endmodule
