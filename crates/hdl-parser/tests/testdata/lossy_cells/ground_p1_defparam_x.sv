module c #(parameter logic [3:0] B = 0) ();
  initial #1 $display("A b=%b", B);
endmodule
module t;
  c u();
  defparam u.B = 4'b1x01;
  initial #100 $finish;
endmodule
