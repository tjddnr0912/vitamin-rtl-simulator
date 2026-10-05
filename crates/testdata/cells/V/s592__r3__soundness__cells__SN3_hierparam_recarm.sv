module top;
  localparam K = 4;
  if (1) begin : gb
    case (8'd4)
      K: begin : g1 localparam P = 1; end
      default: begin : g2 localparam P = 2; end
    endcase
    localparam K = 8;
  end
  initial #1 $display("@P=%0d", gb.g1.P);
  initial #5 $finish;
endmodule
