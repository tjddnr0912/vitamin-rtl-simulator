module top;
  localparam [7:0] Q = 8'd5;
  if (1) begin : gb
    localparam [7:0] P = Q;
    case (32'd3)
      P: begin : g wire [7:0] w = 8'd200; initial #1 $display("@p %0d bits=%0d P=%0d", w, $bits(w), P); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d P=%0d", w, $bits(w), P); end
    endcase
    localparam [7:0] Q = 8'd3;
  end
  initial #5 $finish;
endmodule
