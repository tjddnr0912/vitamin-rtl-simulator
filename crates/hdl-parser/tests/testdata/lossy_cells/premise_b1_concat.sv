module t; logic [69:0] v1, v2;
  initial begin
    v1 = {4'b1010, 'x}; v2 = {3{'z}};
    #1 $display("A v1=%b v2=%b", v1, v2);
  end
  initial #100 $finish;
endmodule
