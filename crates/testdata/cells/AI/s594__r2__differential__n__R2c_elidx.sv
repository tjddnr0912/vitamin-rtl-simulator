module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int fz(input int a); fz = a; endfunction
  localparam logic [7:0] A [2] = '{8'hFC, 8'h03};
  localparam int L = ((X + {N{1'b0}}) == 8'hFC) + (A[fz(0)] & 8'd0);
  initial $display("R: L=%0d", L);
  initial #40 $finish;
endmodule
