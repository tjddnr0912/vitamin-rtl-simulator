module c #(parameter logic [3:0] B = 0) ();
  initial #1 $display("A %m b=%b", B);
endmodule
module t;
  c #(.B(4'b1101)) u0();
  c #(.B(4'b1x01)) u1();
  initial #100 $finish;
endmodule
