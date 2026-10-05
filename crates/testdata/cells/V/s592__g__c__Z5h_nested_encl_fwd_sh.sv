module top;
  localparam logic [7:0] K = 8'd1;
  if (1) begin : gb
    if (1) begin : n
      case (K)
        8'd1: begin : g wire [3:0] w = 4'd9; initial #1 $display("@outer %0d bits=%0d", w, $bits(w)); end
        8'd99: begin : g wire [7:0] w = 8'd200; initial #1 $display("@inner %0d bits=%0d", w, $bits(w)); end
        default: begin : g initial #1 $display("@def"); end
      endcase
    end
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
