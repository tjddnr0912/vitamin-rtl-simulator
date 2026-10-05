module top;
  for (genvar i = 0; i < 3; i++) begin : g
    case (i)
      0: begin : c0 initial $display("N01 c0_%0d", i); end
      65'd1: begin : c1 initial $display("N01 c1_%0d", i); end
      default: begin : cd initial $display("N01 cd_%0d", i); end
    endcase
  end
endmodule
