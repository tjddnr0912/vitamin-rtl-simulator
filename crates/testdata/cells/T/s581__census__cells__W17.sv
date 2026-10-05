module top;
  generate
    case (4'd8 >> 1)
      5'd4: begin : g_a initial $display("W17 a"); end
      default: begin : g_def initial $display("W17 def"); end
    endcase
  endgenerate
endmodule
