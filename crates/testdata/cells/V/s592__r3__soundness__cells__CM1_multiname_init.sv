module top;
  localparam K = 4;
  if (1) begin : gb
    case (8'd4)
      K: begin : g logic [3:0] a, v = 4'd9; end
      default: begin : g logic [7:0] a, v = 8'd200; end
    endcase
    localparam K = 8;
  end
  initial #1 $display("@v=%0d bits=%0d", gb.g.v, $bits(gb.g.v));
  initial #5 $finish;
endmodule
