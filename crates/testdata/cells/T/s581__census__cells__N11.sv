module top;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam logic signed [3:0] Q = i - 1;
    case (Q)
      4'b1111: begin : c0 initial $display("N11 a_%0d", i); end
      default: begin : cd initial $display("N11 def_%0d", i); end
    endcase
  end
endmodule
