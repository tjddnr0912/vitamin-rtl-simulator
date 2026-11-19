module t; function automatic logic [7:0] f(input logic [7:0] a = 'x); return a; endfunction
  localparam logic [7:0] P = f();
  initial begin #1 $display("A p=%b", P); end
  initial #100 $finish;
endmodule
