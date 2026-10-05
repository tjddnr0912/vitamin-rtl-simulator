module top;
  localparam [64:0] Q = 65'h1_0000_0000_0000_0005;
  if (1) begin : gb
    localparam [64:0] P = Q;
    case (32'd3)
      P: begin : g wire [7:0] w = 8'd200; initial #1 $display("@p %0d bits=%0d P=%h", w, $bits(w), P); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d P=%h", w, $bits(w), P); end
    endcase
    localparam [64:0] Q = 3;
  end
  initial #5 $finish;
endmodule
