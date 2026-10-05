module top;
  localparam [64:0] W = 65'd3;
  generate
    case (3)
      W: begin : g_a initial $display("L17 a"); end
      default: begin : g_def initial $display("L17 def"); end
    endcase
  endgenerate
endmodule
