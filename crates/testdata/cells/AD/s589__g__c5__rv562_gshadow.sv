module top;
  localparam int N = 4;
  function automatic logic [N-1:0] g(input int a); g = '1; endfunction
  if (1) begin : b
    localparam real N = 2.5;
    localparam int P = g(0);
    initial #1 $display("P=%0d", P);
  end
  initial #2 $finish;
endmodule
