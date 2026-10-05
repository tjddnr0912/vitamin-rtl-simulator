module t;
  if (1) begin : gb
    case (-1)
      32'hFFFFFFFF: begin : g wire [7:0] w = 8'd200; initial #1 $display("D1W a %0d bits=%0d", w, $bits(w)); end
      K: begin : g wire [7:0] w = 8'd2; initial #1 $display("D1W k %0d", w); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("D1W def %0d bits=%0d", w, $bits(w)); end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
endmodule
