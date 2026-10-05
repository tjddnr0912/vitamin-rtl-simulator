package pa; localparam W = 5; endpackage
package pb; localparam [64:0] W = 65'h1_0000_0000_0000_0007; endpackage
module X2;
  import pa::*;
  import pb::W;
  generate
    case (W)
      65'h1_0000_0000_0000_0007: begin : a initial $display("arm a"); end
      65'd5: begin : five initial $display("arm five"); end
      default: begin : d initial $display("arm def"); end
    endcase
  endgenerate
  initial #1 $finish;
endmodule
