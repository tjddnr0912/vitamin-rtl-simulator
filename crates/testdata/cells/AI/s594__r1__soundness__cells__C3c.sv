module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r; endfunction
  function automatic int fk(input int a); logic [7:0] r; r = 8'd0; fk = r + ((X + {N{1'b0}}) == 8'hFC); endfunction
  localparam int L = fk(0);
  initial $display("L=%0d", L);
  initial #100 $finish;
endmodule
