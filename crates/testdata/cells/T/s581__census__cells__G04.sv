module top;
  localparam logic signed [3:0] i = -1;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i)
      4'b1111: begin : c0 initial $display("G04 a_%0d", i); end
      default: begin : cd initial $display("G04 def_%0d", i); end
    endcase
  end
endmodule
