package pa; localparam W = 5; endpackage
package pb; localparam [64:0] W = 65'h1_0000_0000_0000_0007; endpackage
package pc;
  import pa::*;
  import pb::W;
  localparam [64:0] Z = W;
  localparam [64:0] Y = W + 65'd1;
endpackage
module X4;
  initial begin
    $display("Z=%h Y=%h", pc::Z, pc::Y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
