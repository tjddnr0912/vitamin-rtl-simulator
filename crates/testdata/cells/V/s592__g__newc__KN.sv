module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : x wire [7:0] w = 8'd200; end
    else begin : x wire [3:0] w = 4'd9; end
    localparam integer K = 2;
  end
  initial #1 $display("@w=%0d bits=%0d", g.x.w, $bits(g.x.w));
  initial #5 $finish;
endmodule
