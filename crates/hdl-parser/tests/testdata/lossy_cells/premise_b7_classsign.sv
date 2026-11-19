module t;
  class C #(parameter signed P = 4'hF); function int v(); return P; endfunction endclass
  C c; initial begin c = new; #1 $display("A v=%0d", c.v()); end
  initial #100 $finish;
endmodule
