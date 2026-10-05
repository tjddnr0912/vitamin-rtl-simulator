module top;
  localparam S = 1;
  if (1) begin : gb
    case (S)
      1: begin : g initial #1 $display("@one"); end
      default: begin : g initial #1 $display("@def"); end
    endcase
    localparam real S = 1.5;
  end
  initial #5 $finish;
endmodule
