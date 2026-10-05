module top;
  if (1) begin : gb
    localparam logic signed [3:0] Q = -1;
    case (Q)
      4'b1111: begin : c0 initial $display("N10 a"); end
      default: begin : cd initial $display("N10 def"); end
    endcase
  end
endmodule
