module top;
  wire [7:0] o;
  if (1) begin : gb
    case (-1)
      32'hFFFFFFFF: begin : ga assign o = 8'd1; end
      K: begin : gk assign o = 8'd2; end
      default: begin : gd assign o = 8'd99; end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
  initial #1 $display("D5 o=%0d", o);
endmodule
