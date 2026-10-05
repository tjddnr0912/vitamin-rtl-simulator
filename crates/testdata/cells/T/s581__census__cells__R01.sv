module top;
  localparam M = 1;
  localparam [64:0] LPA = 65'd1;
  generate
    case (M)
      LPA: begin : g_a initial $display("R01 a"); end
      default: begin : g_def initial $display("R01 def"); end
    endcase
  endgenerate
endmodule
