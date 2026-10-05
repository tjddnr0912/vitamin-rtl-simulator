module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r; endfunction
  localparam int L = (((X + {N{1'b0}}) + (8'd1 << (1:fl(2):3))) == 8'hFD);
  localparam int K = (((X + {N{1'b0}}) + (8'd1 << fl(2))) == 8'hFD);
  initial $display("L=%0d K=%0d", L, K);
  initial #100 $finish;
endmodule
