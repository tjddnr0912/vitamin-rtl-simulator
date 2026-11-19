module t; logic [69:0] v; logic a, b, c, d;
  initial begin
    v = 70'h0;
    a = v inside {'x}; b = v inside {'z}; c = v inside {'1}; d = (v ==? 'x);
    #1 $display("A a=%b b=%b c=%b d=%b", a, b, c, d);
  end
  initial #100 $finish;
endmodule
