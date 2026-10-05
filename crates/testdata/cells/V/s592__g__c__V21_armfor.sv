module top;
  if (1) begin : gb
    case (8'd99)
      K: begin : g for (genvar j = 0; j < 2; j++) begin : L wire [7:0] w = 8'd200 + j; initial #1 $display("@k j%0d %0d bits=%0d", j, w, $bits(w)); end end
      default: begin : g for (genvar j = 0; j < 3; j++) begin : L wire [3:0] w = 4'd9; initial #1 $display("@def j%0d %0d bits=%0d", j, w, $bits(w)); end end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
