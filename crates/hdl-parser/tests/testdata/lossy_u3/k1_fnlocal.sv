typedef logic [3:0] let;
module t;
  function automatic int f(); let x; x = 4'd5; return x; endfunction
  initial begin $display("A %0d", f()); $finish; end
endmodule
