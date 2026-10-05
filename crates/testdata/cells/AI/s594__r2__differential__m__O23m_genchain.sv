module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int fz(input int a); fz = a; endfunction
  if (fz(1) == 1) begin : G
    localparam L = X + {N{1'b0}};
    initial $display("R: GL=%0d GB=%0d", L, $bits(L));
  end
  if (fz(0) == 1) begin : H1 initial $display("R: CH=first"); end
  else if ((X + {N{1'b0}}) == 8'hFC) begin : H2 initial $display("R: CH=then"); end
  else begin : H3 initial $display("R: CH=else"); end
  initial #40 $finish;
endmodule
