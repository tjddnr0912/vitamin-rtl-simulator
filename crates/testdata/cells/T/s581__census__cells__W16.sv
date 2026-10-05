module top;
  generate
    case (~4'd0)
      4'hF: begin : g_a initial $display("W16 a"); end
      default: begin : g_def initial $display("W16 def"); end
    endcase
  endgenerate
endmodule
