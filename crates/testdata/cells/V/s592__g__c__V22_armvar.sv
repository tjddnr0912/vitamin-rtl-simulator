module top;
  if (1) begin : gb
    case (8'd99)
      K: begin : g logic [7:0] v = 8'd200; initial #1 $display("@k %0d bits=%0d", v, $bits(v)); end
      default: begin : g logic [3:0] v = 4'd9; initial #1 $display("@def %0d bits=%0d", v, $bits(v)); end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
