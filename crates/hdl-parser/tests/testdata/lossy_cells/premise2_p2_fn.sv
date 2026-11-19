module t;
  logic [69:0] r1, r2, r3, r4, r5, r6, r7, r8;
  function automatic logic [69:0] f1(); logic [69:0] v = 'x; return v; endfunction
  function automatic logic [69:0] f2(); logic [69:0] v; v = 'z; return v; endfunction
  function logic [69:0] f3(); logic [69:0] v; v = 'z; return v; endfunction
  function automatic logic [69:0] f4(input int k); f4 = 'x; endfunction
  function automatic void f5(output logic [69:0] o); o = 'z; endfunction
  task automatic t6(output logic [69:0] o); logic [69:0] v = 'x; #1 o = v; endtask
  function automatic void f7(ref logic [69:0] o); o = 'x; endfunction
  function automatic void f8(inout logic [69:0] o); o = 'z; endfunction
  initial begin r1 = f1(); r2 = f2(); r3 = f3(); r4 = f4(1); f5(r5); #1 $display("A r1=%h r2=%h r3=%h r4=%h r5=%h", r1,r2,r3,r4,r5); end
  initial begin t6(r6); r7 = 0; f7(r7); r8 = 0; f8(r8); begin : blk logic [69:0] bv = 'x; #2 $display("B r6=%h r7=%h r8=%h bv=%h", r6, r7, r8, bv); end end
  initial #100 $finish;
endmodule
