module top;
  localparam [64:0] i = 65'd9;
  for (genvar i = 0; i < 2; i++) begin : g
    case (1)
      i: begin : c0 initial $display("G01 a_%0d", i); end
      default: begin : cd initial $display("G01 def_%0d", i); end
    endcase
  end
endmodule
