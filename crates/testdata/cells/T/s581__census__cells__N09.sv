module top;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i - 1)
      32'hFFFFFFFF: begin : c0 initial $display("N09 a_%0d", i); end
      default: begin : cd initial $display("N09 def_%0d", i); end
    endcase
  end
endmodule
