module t;
  localparam int N = 9;
  if (1) begin : g
    localparam logic [7:0] L = '0 | (((8'd200 + 8'd100) >> 1) + (|{N{8'hFF}}));
  end
  initial $display("L=%0d", g.L);
  initial #100 $finish;
endmodule
