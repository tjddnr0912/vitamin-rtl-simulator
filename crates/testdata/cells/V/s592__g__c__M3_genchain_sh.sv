module top;
  localparam Q = 7;
  if (1) begin : gb
    localparam P = Q;
    case (P)
      7: begin : g wire [7:0] w = 8'd7; initial #1 $display("@seven %0d bits=%0d P=%0d", w, $bits(w), P); end
      5: begin : g wire [5:0] w = 6'd5; initial #1 $display("@five %0d bits=%0d P=%0d", w, $bits(w), P); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d P=%0d", w, $bits(w), P); end
    endcase
    localparam Q = R;
    localparam R = 5;
  end
  initial #5 $finish;
endmodule
