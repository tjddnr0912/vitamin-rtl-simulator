module top;
  wire [3:0] w = 4'd1;
  generate
    case (1)
      w, 1: begin : g_a initial $display("M12 a"); end
      default: begin : g_def initial $display("M12 def"); end
    endcase
  endgenerate
endmodule
