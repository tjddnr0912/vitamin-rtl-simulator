module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r; endfunction
  localparam int K = 8'd0 + (8'd1 << ((X + {N{1'b0}}) >> 7));
  initial $display("K=%0d", K);
  initial #100 $finish;
endmodule
