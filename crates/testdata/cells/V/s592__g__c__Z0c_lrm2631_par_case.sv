package p; localparam logic [7:0] X = 8'd1; endpackage
module top;
  import p::*;
  if (1) begin : b
    case (X)
      8'd1: begin : g wire [3:0] w = 4'd9; initial #1 $display("@pkg %0d bits=%0d", w, $bits(w)); end
      8'd2: begin : g wire [7:0] w = 8'd200; initial #1 $display("@local %0d bits=%0d", w, $bits(w)); end
      default: begin : g initial #1 $display("@def"); end
    endcase
    localparam logic [7:0] X = 8'd2;
  end
  initial #5 $finish;
endmodule
