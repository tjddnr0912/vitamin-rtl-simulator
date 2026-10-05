`timescale 1ns/1ns
module leaf #(parameter int K = 0) (); initial #2 $display("@leaf %m %0d", K); endmodule
module top;
  for (genvar i = 0; i < 3; i++) begin : g
    localparam logic [64:0] LW = {1'b1, 62'd0, 2'(i)};
    case (2'(i))
      LW[1:0]: begin : c leaf #(.K(10 + i)) u (); end
      default: begin : c leaf #(.K(20 + i)) u (); end
    endcase
    case (i)
      LW[64:63] - 2'd2: begin : e initial #1 $display("@e%0d LWtop", i); end
      default: begin : e initial #1 $display("@e%0d def", i); end
    endcase
  end
endmodule
