module t; logic [69:0] q[$]; logic [69:0] d[]; logic [69:0] aa[int]; logic [69:0] r1, r2, r3, r4;
  function automatic logic [69:0] f(input logic c); return c ? 'z : 'x; endfunction
  function automatic logic [69:0] g(input logic [69:0] a); return a; endfunction
  function automatic void h(output logic [69:0] o); o = '1; endfunction
  initial begin q.push_back('1); q.push_back('z); d = new[2]; d[0] = '1; d[1] = 'z; aa[3] = 'z; r1 = f(1); r2 = g(g('1)); h(r3); r4 = g(1 ? 'z : '1);
    #1 $display("A q0=%h q1=%h d0=%h d1=%h", q[0], q[1], d[0], d[1]); $display("B aa=%h r1=%h r2=%h r3=%h r4=%h", aa[3], r1, r2, r3, r4); end
  initial #100 $finish;
endmodule
