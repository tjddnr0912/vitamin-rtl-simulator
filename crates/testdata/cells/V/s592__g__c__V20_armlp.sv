module top;
  if (1) begin : gb
    case (8'd99)
      K: begin : g localparam W = 8; wire [W-1:0] w = 8'd200; initial #1 $display("@k %0d bits=%0d W=%0d", w, $bits(w), W); end
      default: begin : g localparam W = 4; wire [W-1:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d W=%0d", w, $bits(w), W); end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
