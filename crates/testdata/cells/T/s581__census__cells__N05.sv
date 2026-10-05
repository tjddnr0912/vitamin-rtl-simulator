module top;
  localparam logic [3:0] L = 4'd5;
  if (1) begin : gb
    localparam [64:0] L = 65'd7;
    case (5)
      L: begin : c0 initial $display("N05 a"); end
      default: begin : cd initial $display("N05 def"); end
    endcase
  end
endmodule
