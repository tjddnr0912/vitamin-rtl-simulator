module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int fz(input int a); fz = a; endfunction
  localparam int L = (fz(1) != 0) ? ((X + {N{1'b0}}) == 8'hFC) : 0;
  initial $display("R: L=%0d", L);
  initial #40 $finish;
endmodule
