module top;
  generate
    case ('1)
      4'b1111: begin : g_a initial $display("F07 a"); end
      default: begin : g_def initial $display("F07 def"); end
    endcase
  endgenerate
endmodule
