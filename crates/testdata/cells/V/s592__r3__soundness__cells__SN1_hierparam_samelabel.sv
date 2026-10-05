module top;
  localparam K = 4;
  if (1) begin : gb
    case (8'd4)
      K: begin : g localparam P = 1; end
      default: begin : g localparam P = 2; end
    endcase
    localparam K = 8;
  end
  initial #1 $display("@P=%0d", gb.g.P);
  initial #5 $finish;
endmodule
