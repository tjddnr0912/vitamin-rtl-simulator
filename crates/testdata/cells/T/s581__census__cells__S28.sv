module top;
  generate
    case ($signed(4'b1111))
      4'b1111: begin : g_a initial $display("S28 a"); end
      default: begin : g_def initial $display("S28 def"); end
    endcase
  endgenerate
endmodule
