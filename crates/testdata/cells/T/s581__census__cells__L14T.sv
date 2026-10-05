module top;
  localparam P = 3;
  generate
    case (4)
      top.P: begin : g_a initial $display("L14T a"); end
      default: begin : g_def initial $display("L14T def"); end
    endcase
  endgenerate
endmodule
