module top;
  generate
    case (1)
      $isunknown(4'bx100 ==? 4'b1?00): begin : g_a initial $display("T2 a"); end
      default: begin : g_def initial $display("T2 def"); end
    endcase
  endgenerate
endmodule
