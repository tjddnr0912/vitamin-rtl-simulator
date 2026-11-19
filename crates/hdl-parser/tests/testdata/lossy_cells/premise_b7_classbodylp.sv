module t;
  class D; localparam unsigned U = 4'sd8; function int v(); return U; endfunction endclass
  D d; initial begin d = new; #1 $display("A v=%0d", d.v()); end
  initial #100 $finish;
endmodule
