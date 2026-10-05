module top;
  if (1) begin : gb
    case (-1)
      32'hFFFFFFFF: begin : blk wire [7:0] w = 8'd1; initial #1 $display("D4 a %0d", w); end
      K: begin : blk wire [7:0] w = 8'd2; initial #1 $display("D4 k %0d", w); end
      default: begin : blk wire [7:0] w = 8'd99; initial #1 $display("D4 def %0d", w); end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
endmodule
