module top;
  localparam [64:0] W = {1'b1, 64'd0};
  generate
    case (0)
      W >> 64: begin : g_a initial $display("L29 a"); end
      0: begin : g_b initial $display("L29 b"); end
      default: begin : g_def initial $display("L29 def"); end
    endcase
  endgenerate
endmodule
