module top;
  localparam [64:0] K = 65'h1_0000_0000_0000_0005;
  if (1) begin : gb
    case (8'd5)
      K: begin : g wire [7:0] w = 8'd200; initial #1 $display("@k %0d bits=%0d K=%h", w, $bits(w), K); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d K=%h", w, $bits(w), K); end
    endcase
    localparam [7:0] K = 8'd5;
  end
  initial #5 $finish;
endmodule
