module top;
  for (genvar i = 0; i < 3; i++) begin : g
    localparam [64:0] LG = i;
    case (1)
      LG: begin : c0 initial $display("N07 a_%0d", i); end
      default: begin : cd initial $display("N07 def_%0d", i); end
    endcase
  end
endmodule
