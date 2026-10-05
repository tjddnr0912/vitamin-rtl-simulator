module top;
  int cnt = 0;
  function automatic int f(input int a);
    cnt = cnt + 1;
    f = a + 5;
  endfunction
  logic [15:0][3:0] m = 0;
  initial begin #1 m[f(2)] = 4'hA; $display("m=%h cnt=%0d", m, cnt); $finish; end
endmodule
