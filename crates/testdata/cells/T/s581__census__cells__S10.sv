module top;
  generate
    case (4'sb1111)
      8'd255: begin : g_a initial $display("S10 a"); end
      default: begin : g_def initial $display("S10 def"); end
    endcase
  endgenerate
endmodule
