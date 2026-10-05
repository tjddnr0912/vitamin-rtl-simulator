module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic int fk(input int a); logic [7:0] r; int u; r = 8'd0; u = ((X + {N{1'b0}}) == 8'hFC); fk = r + u; endfunction
  localparam int L = fk(0);
  initial $display("L=%0d", L);
  initial #100 $finish;
endmodule
