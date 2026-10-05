`timescale 1ns/1ns
module leaf #(parameter int K = 0) (output logic [7:0] y); assign y = K; endmodule
module top;
  for (genvar i = 0; i < 3; i++) begin : g
    localparam logic [64:0] LW = {1'b1, 62'd0, 2'(i)};
    case (LW[1:0])
      2'd0: begin : c leaf #(.K(10)) u (.y()); end
      2'd1: begin : c leaf #(.K(11)) u (.y()); end
      default: begin : c leaf #(.K(12 + i)) u (.y()); end
    endcase
    case (i)
      LW[64:63] - 2'd2: begin : e initial #1 $display("@e%0d LWtop", i); end
      default: begin : e initial #1 $display("@e%0d def", i); end
    endcase
    initial #2 $display("@g%0d %0d", i, g[i].c.u.y);
  end
endmodule
