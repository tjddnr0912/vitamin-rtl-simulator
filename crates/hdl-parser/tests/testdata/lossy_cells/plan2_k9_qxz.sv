module t; logic [69:0] q[$]; logic [69:0] p[$];
  initial begin q.push_back('x); q.push_front('z); q.insert(1, 'z); q.push_back(70'bz); p = {'z, 'x}; p[0] = 'z; p.push_back(1 ? 'z : 'x);
    #1 $display("A q0=%h q1=%h q2=%h q3=%h", q[0], q[1], q[2], q[3]); $display("B p0=%h p1=%h p2=%h", p[0], p[1], p[2]); end
  initial #100 $finish;
endmodule
