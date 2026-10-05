module top;
  localparam [64:0] P = {1'b1, 64'd0};
  generate
    case (1)
      P > 0: begin : g_a initial $display("L28 a"); end
      default: begin : g_def initial $display("L28 def"); end
    endcase
  endgenerate
endmodule
