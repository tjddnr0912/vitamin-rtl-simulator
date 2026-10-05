module top;
  parameter string SS = "ab";
  generate
    case (16'h6162)
      SS: begin : g_a initial $display("L08 a"); end
      default: begin : g_def initial $display("L08 def"); end
    endcase
  endgenerate
endmodule
