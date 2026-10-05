module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r; endfunction
  integer k = 0;
  initial begin repeat (fl(0) + ((X + {N{1'b0}}) == 8'hFC)) k = k + 1; $display("k=%0d", k); end
  initial #100 $finish;
endmodule
