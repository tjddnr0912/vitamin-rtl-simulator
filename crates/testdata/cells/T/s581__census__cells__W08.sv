module top;
  generate
    case (5'd16)
      4'd15 + 4'd1: begin : g_a initial $display("W08 a"); end
      default: begin : g_def initial $display("W08 def"); end
    endcase
  endgenerate
endmodule
