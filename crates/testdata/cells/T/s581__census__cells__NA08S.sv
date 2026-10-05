module top;
  for (genvar i = 0; i < 2; i++) begin : gi
    for (genvar j = 0; j < 2; j++) begin : gj
      case (i * 2 + j)
        65'd1: begin : a wire [7:0] w = 8'd1; initial #1 $display("NA08S one_%0d%0d %0d", i, j, w); end
        65'd2: begin : b wire [7:0] w = 8'd2; initial #1 $display("NA08S two_%0d%0d %0d", i, j, w); end
        default: begin : d wire [7:0] w = 8'd9; initial #1 $display("NA08S d_%0d%0d %0d", i, j, w); end
      endcase
    end
  end
endmodule
