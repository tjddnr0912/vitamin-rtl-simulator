module top;
  for (genvar i = 0; i < 3; i++) begin : g
    case (i)
      {64'd0, 1'b1}: begin : c0 initial $display("N08 a_%0d", i); end
      default: begin : cd initial $display("N08 def_%0d", i); end
    endcase
  end
endmodule
