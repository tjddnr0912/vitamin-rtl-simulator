module top;
  localparam M = 1;
  localparam [64:0] LPA = 65'd2;
  generate
    case (M)
      LPA: begin : g_a initial $display("R02 a"); end
      1: begin : g_b initial $display("R02 b"); end
      default: begin : g_def initial $display("R02 def"); end
    endcase
  endgenerate
endmodule
