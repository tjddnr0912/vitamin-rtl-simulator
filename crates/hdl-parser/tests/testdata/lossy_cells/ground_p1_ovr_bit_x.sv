module c #(parameter bit [3:0] B = 0) ();
  initial #1 $display("A b=%b", B);
endmodule
module t;
  c #(.B(4'b1x01)) u();
  initial #100 $finish;
endmodule
