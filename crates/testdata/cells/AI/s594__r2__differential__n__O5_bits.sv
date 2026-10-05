module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int fz(input int a); fz = a; endfunction
  localparam L0 = X + {N{1'b0}};
  localparam int M = fz(0) + $bits(L0);
  localparam int M2 = $bits(L0);
  initial $display("R: B=%0d M=%0d M2=%0d", $bits(L0), M, M2);
  initial #40 $finish;
endmodule
