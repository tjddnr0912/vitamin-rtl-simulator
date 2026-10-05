module top;
  localparam [64:0] W = {1'b1, 64'd1};
  generate
    case (1)
      W: begin : g_a initial $display("L18 a"); end
      default: begin : g_def initial $display("L18 def"); end
    endcase
  endgenerate
endmodule
