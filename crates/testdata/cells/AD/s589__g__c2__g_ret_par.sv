module top;
  localparam int W = 7;
  if (1) begin : g
    localparam int W = 3;
    function automatic logic [W:0] h(input int x); return x; endfunction
    localparam int P = h(1000);
    int v;
    initial begin v = h(1000); #1 $display("P=%0d v=%0d", P, v); end
  end
  initial #3 $finish;
endmodule
