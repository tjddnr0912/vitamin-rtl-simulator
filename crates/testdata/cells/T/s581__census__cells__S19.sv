module top;
  generate
    case (65'h1FFFFFFFFFFFFFFFF)
      -1: begin : g_a initial $display("S19 a"); end
      default: begin : g_def initial $display("S19 def"); end
    endcase
  endgenerate
endmodule
