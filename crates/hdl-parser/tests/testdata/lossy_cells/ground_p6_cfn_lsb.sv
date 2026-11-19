module t;
  function automatic logic [7:0] g(); bit [5:2] x; x = 0; x[2] = 1; x[3] = 1; return x; endfunction
  localparam logic [7:0] V = g();
  initial begin #1 $display("A v=%b", V); end
  initial #100 $finish;
endmodule
