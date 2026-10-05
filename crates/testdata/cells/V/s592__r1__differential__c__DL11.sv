module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : a
      wire [7:0] w;
    end else begin : b
      wire [3:0] w;
    end
    localparam integer K = 2;
  end
  assign g.b.w = 4'd9;
  initial #1 $display("@w=%0d bits=%0d", g.b.w, $bits(g.b.w));
  initial #10 $finish;
endmodule
