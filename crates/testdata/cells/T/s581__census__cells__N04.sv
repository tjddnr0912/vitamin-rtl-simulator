module top;
  localparam [64:0] L = 65'd5;
  if (1) begin : gb
    localparam logic [3:0] L = 4'd7;
    case (7)
      L: begin : c0 initial $display("N04 a"); end
      default: begin : cd initial $display("N04 def"); end
    endcase
  end
endmodule
