module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic int f1(input int a); return 1; endfunction
  if (f1(0) == 1) begin : g localparam U = X + {N{1'b0}}; end
  initial $display("U=%0d B=%0d", g.U, $bits(g.U));
  initial #100 $finish;
endmodule
