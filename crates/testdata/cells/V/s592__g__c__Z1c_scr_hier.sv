module top;
  if (1) begin : gb
    case (S)
      8'd99: begin : g wire [7:0] w = 8'd200; end
      default: begin : g wire [3:0] w = 4'd9; end
    endcase
    localparam logic [7:0] S = 8'd99;
  end
  initial #1 $display("@h w=%0d bits=%0d", gb.g.w, $bits(gb.g.w));
  initial #5 $finish;
endmodule
