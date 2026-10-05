module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r; endfunction
  function automatic int f(input int a); f = ((X + {N{1'b0}}) == 8'hFC); endfunction
  if ((fl(0) + f(0)) > 0) begin : g initial $display("GI=then"); end
  else begin : g initial $display("GI=else"); end
  initial #100 $finish;
endmodule
