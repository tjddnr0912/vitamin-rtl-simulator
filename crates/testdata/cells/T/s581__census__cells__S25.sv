module top;
  generate
    case (4'd15)
      8'sb00001111: begin : g_a initial $display("S25 a"); end
      default: begin : g_def initial $display("S25 def"); end
    endcase
  endgenerate
endmodule
