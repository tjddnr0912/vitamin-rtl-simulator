module top;
  for (genvar i = 0; i < 3; i++) begin : g
    localparam [64:0] L = 65'd1 + i;
    case (2)
      L: begin : c0 initial $display("N02 a_%0d", i); end
      default: begin : cd initial $display("N02 def_%0d", i); end
    endcase
  end
endmodule
