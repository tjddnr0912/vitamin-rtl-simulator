module t; typedef enum logic [69:0] {EA = 'x, EB = 70'd1} e_t; e_t v;
  initial begin v = EA; #1 $display("A v=%h", v); end
  initial #100 $finish;
endmodule
