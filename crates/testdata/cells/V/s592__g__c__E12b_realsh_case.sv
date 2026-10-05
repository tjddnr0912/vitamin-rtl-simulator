module top;
  localparam real Q = 2.5;
  if (1) begin : gb
    localparam P = Q;
    case (P)
      7: begin : g wire [7:0] w = 8'd200; initial #1 $display("@seven %0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d", w, $bits(w)); end
    endcase
    localparam integer Q = 7;
  end
  initial #5 $finish;
endmodule
