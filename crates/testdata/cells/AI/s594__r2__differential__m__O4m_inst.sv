module sub #(parameter int P = 0) ();
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  localparam L = X + {N{1'b0}};
  initial $display("R: P=%0d L=%0d B=%0d", P, L, $bits(L));
endmodule
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int fz(input int a); fz = a; endfunction
  sub #(.P(fz(5))) u();
  initial #40 $finish;
endmodule
