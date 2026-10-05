module top;
  localparam logic [7:0] P = 8'h62;
  generate
    case (P)
      "a" + 1: begin : g_a initial $display("L06 a"); end
      default: begin : g_def initial $display("L06 def"); end
    endcase
  endgenerate
endmodule
