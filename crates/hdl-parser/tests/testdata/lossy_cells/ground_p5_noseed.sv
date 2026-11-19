module t;
  function automatic logic [39:0] cf(); logic [39:0] r; r = 40'h12; return r; endfunction
  localparam logic [39:0] P = cf();
  initial begin #1 $display("A p=%h", P); end
  initial #100 $finish;
endmodule
