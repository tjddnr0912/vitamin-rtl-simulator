module top;
  localparam integer K = 1;
  if (1) begin : g
    case (K)
      2: begin : a logic [7:0] v; end
      default: begin : d logic [3:0] v; end
    endcase
    localparam integer K = 2;
  end
  initial #1 $display("@bits=%0d", $bits(g.d.v));
  initial #10 $finish;
endmodule
