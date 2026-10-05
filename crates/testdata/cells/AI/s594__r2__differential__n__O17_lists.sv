package pk;
  function automatic int fz(input int a); fz = a; endfunction
endpackage
module sub #(parameter logic signed [7:0] XS = -4, parameter int NN = 2, parameter int P = pk::fz(1),
             parameter int Q = ((XS + {NN{1'b0}}) == 8'hFC)) ();
  initial #1 $display("R: P=%0d Q=%0d", P, Q);
endmodule
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int fz(input int a); fz = a; endfunction
  localparam int A1 = fz(3), L = ((X + {N{1'b0}}) == 8'hFC);
  sub u();
  initial $display("R: A1=%0d L=%0d", A1, L);
  initial #40 $finish;
endmodule
