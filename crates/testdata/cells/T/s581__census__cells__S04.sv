module top;
  generate
    case ("ab")
      16'h6162: begin : g_a initial $display("S04 a"); end
      default: begin : g_def initial $display("S04 def"); end
    endcase
  endgenerate
endmodule
