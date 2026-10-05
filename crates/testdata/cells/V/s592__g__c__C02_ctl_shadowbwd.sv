module top;
  localparam K = 4;
  if (1) begin : gb
    localparam K = 8;
    wire [K-1:0] w = '1;
    case (K)
      8: begin : g initial #1 $display("@eight bits=%0d K=%0d", $bits(w), K); end
      default: begin : g initial #1 $display("@def bits=%0d K=%0d", $bits(w), K); end
    endcase
  end
  initial #5 $finish;
endmodule
