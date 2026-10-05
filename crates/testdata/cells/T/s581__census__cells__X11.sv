module top;
  generate
    case (1)
      (4'b1100 ==? 4'b1?00): begin : g_a initial $display("X11 a"); end
      default: begin : g_def initial $display("X11 def"); end
    endcase
  endgenerate
endmodule
