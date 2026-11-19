class C;
  logic [69:0] f = 'x;
  logic [69:0] s = 'z;
  logic [69:0] h; logic [69:0] h2;
  function new(); h = 'z; endfunction
  function void m(); logic [69:0] v = 'z; this.f = 'z; h2 = v; endfunction
  function logic [69:0] rf(); rf = 'x; endfunction
endclass
module t; C o; logic [69:0] r1;
  initial begin o = new; #1 $display("A f=%h s=%h h=%h", o.f, o.s, o.h);
    o.m(); r1 = o.rf(); #1 $display("B f=%h h2=%h r1=%h", o.f, o.h2, r1); end
  initial #100 $finish;
endmodule
