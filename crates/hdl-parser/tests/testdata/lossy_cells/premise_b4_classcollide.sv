module t;
  class C #(parameter logic [7:0] P = 0); function logic [7:0] v(); return P; endfunction endclass
  C #(8'b0000101) a; C #(8'b0000_1?01) b;
  initial begin a = new; b = new; #1 $display("A a=%b b=%b", a.v(), b.v()); end
  initial #100 $finish;
endmodule
