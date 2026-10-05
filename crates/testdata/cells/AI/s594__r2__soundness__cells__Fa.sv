module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r; endfunction
  localparam int L = ((X + {N{1'b0}}) == 8'hFC) + fl(0);
  initial $display("L=%0d", L);
  initial #100 $finish;
endmodule
