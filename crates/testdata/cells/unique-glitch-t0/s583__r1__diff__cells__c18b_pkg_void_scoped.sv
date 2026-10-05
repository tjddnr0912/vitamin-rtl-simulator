package pk;
  int cnt;
  function automatic void fv(input logic x, input logic z);
    unique if (x) cnt = 1; else if (z) cnt = 2;
  endfunction
endpackage
module top;
  logic p, q;
  initial begin
    p = 0; q = 0;
    #1 pk::fv(p, q); $display("t=%0t pk::fv cnt=%0d", $time, pk::cnt);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
