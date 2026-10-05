module top;
  localparam [64:0] W = {1'b1, 64'd0};
  generate
    case (1)
      W >> 64: begin : g_a initial $display("L30 a"); end
      default: begin : g_def initial $display("L30 def"); end
    endcase
  endgenerate
endmodule
