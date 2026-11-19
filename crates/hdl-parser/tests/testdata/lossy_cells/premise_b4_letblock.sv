module t;
  function automatic int f(); let lf(logic [3:0] x) = x + 1'b1; return lf(5'h1E); endfunction
  initial begin #1 $display("A f=%0d", f()); end
  initial #100 $finish;
endmodule
