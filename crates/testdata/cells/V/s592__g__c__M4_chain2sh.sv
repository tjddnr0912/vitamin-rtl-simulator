module top;
  localparam Q = 7;
  localparam R = 9;
  if (1) begin : gb
    localparam P = Q;
    case (P)
      7: begin : g initial #1 $display("@seven P=%0d", P); end
      9: begin : g initial #1 $display("@nine P=%0d", P); end
      5: begin : g initial #1 $display("@five P=%0d", P); end
      default: begin : g initial #1 $display("@def P=%0d", P); end
    endcase
    localparam Q = R;
    localparam R = 5;
  end
  initial #5 $finish;
endmodule
