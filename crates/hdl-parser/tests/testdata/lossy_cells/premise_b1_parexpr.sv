module t;
  localparam logic L = (4'b1010 == 'x);
  localparam logic [7:0] P2 = 8'h0F | 'z;
  initial begin #1 $display("A l=%b p2=%b", L, P2); end
  initial #100 $finish;
endmodule
