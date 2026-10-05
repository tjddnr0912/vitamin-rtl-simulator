module top;
  localparam Q = 1;
  if (1) begin : gb
    case (8'd4)
      4: begin : g localparam P = Q; end
      default: begin : g localparam P = 2; end
    endcase
    localparam Q = 9;
  end
  initial #1 $display("@P=%0d", gb.g.P);
  initial #5 $finish;
endmodule
