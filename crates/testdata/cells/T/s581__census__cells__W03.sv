module top;
  generate
    case (4'd15 + 4'd1)
      4'd0: begin : g_a initial $display("W03 a"); end
      default: begin : g_def initial $display("W03 def"); end
    endcase
  endgenerate
endmodule
