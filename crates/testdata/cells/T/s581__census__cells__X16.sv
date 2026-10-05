module top;
  generate
    case (0)
      |4'b000x: begin : g_a initial $display("X16 a"); end
      default: begin : g_def initial $display("X16 def"); end
    endcase
  endgenerate
endmodule
