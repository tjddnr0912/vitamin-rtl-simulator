module top;
  if (1) begin : gb
    case (-72'sd3)
      K: begin : g wire [7:0] w = 8'd200; initial #1 $display("@k %0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d", w, $bits(w)); end
    endcase
    localparam logic signed [71:0] K = -72'sd3;
  end
  initial #5 $finish;
endmodule
