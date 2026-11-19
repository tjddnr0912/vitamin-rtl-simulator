module t;
  function automatic logic [7:0] g(); bit [3:0] x; x = 0; x[0] = 1; x[1] = 1; return x; endfunction
  localparam logic [7:0] V = g();
  initial begin #1 $display("A v=%b", V); end
  initial #100 $finish;
endmodule
