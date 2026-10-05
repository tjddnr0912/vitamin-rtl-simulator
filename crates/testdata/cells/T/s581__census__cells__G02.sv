module top;
  localparam [64:0] i = 65'd9;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i)
      65'd9: begin : c0 initial $display("G02 nine_%0d", i); end
      65'd1: begin : c1 initial $display("G02 one_%0d", i); end
      default: begin : cd initial $display("G02 def_%0d", i); end
    endcase
  end
endmodule
