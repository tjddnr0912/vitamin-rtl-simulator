module top;
  wire [3:0] w = 4'd1;
  generate
    case (1)
      w: begin : g_a initial $display("L15 a"); end
      default: begin : g_def initial $display("L15 def"); end
    endcase
  endgenerate
endmodule
