module top;
  localparam [64:0] W = {1'b1, 64'd0};
  if (1) begin : gb
    localparam logic [3:0] W = 4'd1;
    case (1)
      W >> 0: begin : c0 initial $display("G03 a"); end
      default: begin : cd initial $display("G03 def"); end
    endcase
  end
endmodule
