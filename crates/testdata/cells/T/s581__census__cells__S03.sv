module top;
  localparam [64:0] W = {1'b1, 64'd0};
  generate
    case (W)
      W: begin : g_a initial $display("S03 a"); end
      default: begin : g_def initial $display("S03 def"); end
    endcase
  endgenerate
endmodule
