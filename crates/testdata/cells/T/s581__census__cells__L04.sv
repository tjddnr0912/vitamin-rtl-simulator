module top;
  localparam logic [15:0] P16 = 16'h6162;
  generate
    case (P16)
      "ab": begin : g_a initial $display("L04 a"); end
      default: begin : g_def initial $display("L04 def"); end
    endcase
  endgenerate
endmodule
