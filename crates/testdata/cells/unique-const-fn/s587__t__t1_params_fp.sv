package pk;
  function automatic int pf(input int a);
    pf = 7;
    if (a == 1) pf = 10;
  endfunction
endpackage
module sub #(parameter int W = f(2)) ();
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  initial #1 $display("W=%0d", W);
endmodule
module top;
  import pk::*;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  function automatic int pr(input int a);
    pr = 3;
    if (a == 1) pr = 30;
  endfunction
  function automatic int g(input int a);
    g = f(a) + 1;
  endfunction
  function automatic int lp(input int n);
    lp = 0;
    for (int i = 0; i < n; i++) begin
      lp = lp + 1;
      if (i == 100) lp = 0;
    end
  endfunction
  function automatic logic [3:0] f4(input int a);
    f4 = 7;
    if (a == 1) f4 = 10;
  endfunction
  function automatic int u0(input int a);
    u0 = 7;
    unique0 if (a == 1) u0 = 10;
  endfunction
  localparam int A = f(2);
  parameter int B = f(2);
  localparam int C = $clog2(f(2));
  localparam int D = g(2);
  localparam int E = lp(3);
  localparam int F = pk::pf(2);
  localparam int G = pf(2);
  localparam H = f4(2);
  localparam int I = pr(2);
  localparam int J = u0(2);
  localparam int N = f(1);
  if (1) begin : gb
    localparam int L = f(2);
  end
  sub u ();
  initial begin
    #1 $display("A=%0d B=%0d C=%0d D=%0d E=%0d F=%0d G=%0d", A, B, C, D, E, F, G);
    $display("H=%0d bH=%0d I=%0d J=%0d N=%0d L=%0d", H, $bits(H), I, J, N, gb.L);
    #1 $finish;
  end
endmodule
