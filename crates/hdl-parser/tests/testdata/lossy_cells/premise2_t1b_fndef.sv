module t;
  function automatic logic [7:0] cd(input logic [7:0] a = 8'b1z01_0000); return a; endfunction
  localparam logic [7:0] Q = cd();
  initial begin #1 $display("A q=%b", Q); end
  initial #100 $finish;
endmodule
