package pk;
  localparam [64:0] W = 65'h1_0000_0000_0000_0000;
endpackage
module top;
  import pk::W;
  for (genvar W = 0; W < 2; W++) begin : g
    case (1)
      W: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("B3X a %m %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("B3X def %m %0d", w); end
    endcase
  end
endmodule
