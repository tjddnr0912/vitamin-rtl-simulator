module leaf; logic [69:0] x; endmodule
module t;
  function automatic int cf(input int n); return n; endfunction
  localparam int L = cf(70);
  leaf u();
  for (genvar gi = 0; gi < 1; gi++) begin : gb localparam int K = 40; end
  logic [L-1:0] a = 'x;
  logic [69:0] b, c, d, e;
  logic [cf(70)-1:0] f = 'z;
  initial begin b = 0; c = 0; d = 0; e = 0; b[cf(40)-1:0] = 'x; e[cf(40)-1:0] <= 'z;
    #1 $display("A a=%h f=%h", a, f); $display("B b=%h c=%h d=%h e=%h", b, c, d, e); end
  initial #100 $finish;
endmodule
