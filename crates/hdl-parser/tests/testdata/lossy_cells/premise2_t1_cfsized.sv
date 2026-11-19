module t;
  function automatic logic [7:0] cx(); return 8'b1x01_0000; endfunction
  localparam logic [7:0] P = cx();
  initial begin #1 $display("A p=%b", P); end
  initial #100 $finish;
endmodule
