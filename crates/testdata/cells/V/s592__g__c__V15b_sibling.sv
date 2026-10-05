module top;
  localparam logic [7:0] K = 8'd5;
  if (1) begin : ga
    case (8'd99)
      K: begin : g wire [7:0] w = 8'd200; initial #1 $display("@k %0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d", w, $bits(w)); end
    endcase
  end
  if (1) begin : gb
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
