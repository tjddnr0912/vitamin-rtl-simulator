module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : x logic [7:0] v = 8'd200; end
    else begin : x logic [3:0] v = 4'd9; end
    localparam integer K = 2;
  end
  initial #1 $display("@v=%0d bits=%0d", g.x.v, $bits(g.x.v));
  initial #5 $finish;
endmodule
