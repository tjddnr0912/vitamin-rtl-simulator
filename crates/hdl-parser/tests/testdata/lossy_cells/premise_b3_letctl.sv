module t; logic [7:0] a, b; logic [8:0] w1; logic [69:0] w2, w3; logic [8:0] w4;
  let S = a + b; let L1 = '1; let LZ = 'z; let G(q) = q + 1'b1;
  initial begin a = 8'hFF; b = 8'h01; w1 = S; w2 = L1; w3 = LZ; w4 = G(a); #1 $display("A w1=%h w2=%h w3=%h w4=%h", w1, w2, w3, w4); end
  initial #100 $finish;
endmodule
