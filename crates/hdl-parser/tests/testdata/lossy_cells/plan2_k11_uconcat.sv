module t; logic [69:0] q[$]; logic [69:0] q2[$] = {'1}; logic [69:0] d[]; logic [69:0] m[2]; logic [69:0] n[2];
  initial begin q = {q, '1}; q = {'z, q}; q[$+1] = '1; d = {'1, 'z}; m = {'1, 'z}; n = '{'1, 'z};
    #1 $display("A q0=%h q1=%h q2=%h i0=%h", q[0], q[1], q[2], q2[0]); $display("B d0=%h d1=%h m0=%h m1=%h n0=%h n1=%h", d[0], d[1], m[0], m[1], n[0], n[1]); end
  initial #100 $finish;
endmodule
