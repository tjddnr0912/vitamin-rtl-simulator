module top;
  wire [3:0] v;
  if (1) begin : gb
    wire [7:0] v;
    localparam P = $bits(v);
    case (P)
      8: begin : g wire [7:0] w = 8'd200; initial #1 $display("@eight %0d bits=%0d P=%0d", w, $bits(w), P); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d P=%0d", w, $bits(w), P); end
    endcase
  end
  initial #5 $finish;
endmodule
