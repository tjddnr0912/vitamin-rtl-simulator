module top;
  if (1) begin : gb
    case (K[64:0])
      65'h1_0000_0000_0000_0005: begin : g wire [7:0] w = 8'd200; initial #1 $display("@k %0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d", w, $bits(w)); end
    endcase
    localparam [127:0] K = {64'h1, 64'h5};
  end
  initial #5 $finish;
endmodule
