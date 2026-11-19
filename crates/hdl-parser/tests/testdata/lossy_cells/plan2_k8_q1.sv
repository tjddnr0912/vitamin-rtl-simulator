module t; logic [69:0] q[$]; logic [69:0] p[$]; logic [69:0] e;
  initial begin q.push_back('1); q.push_front('1); q.insert(1, '1); q.push_back(70'h3fffffffffffffffff); e = '1; q.push_back(e); p = {'1, '1}; p[0] = '1;
    #1 $display("A q0=%h q1=%h q2=%h q3=%h q4=%h", q[0], q[1], q[2], q[3], q[4]); $display("B p0=%h p1=%h n=%0d", p[0], p[1], p.size()); end
  initial #100 $finish;
endmodule
