module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : a
      genvar j;
      typedef logic [7:0] t_t;
      t_t w;
    end else begin : b
      genvar j;
      typedef logic [3:0] t_t;
      t_t w;
    end
    localparam integer K = 2;
  end
  initial #1 $display("@bits=%0d", $bits(g.b.w));
  initial #10 $finish;
endmodule
