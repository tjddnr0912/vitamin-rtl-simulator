module top;
  for (genvar i = 0; i < 2; i++) begin : gi
    for (genvar j = 0; j < 2; j++) begin : gj
      localparam [64:0] L = i * 2 + j;
      case (1)
        L: begin : a wire [7:0] w = 8'd1; initial #1 $display("NA08L a_%0d%0d %0d", i, j, w); end
        default: begin : d wire [7:0] w = 8'd9; initial #1 $display("NA08L d_%0d%0d %0d", i, j, w); end
      endcase
    end
  end
endmodule
