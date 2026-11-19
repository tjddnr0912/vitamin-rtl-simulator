module sub; function automatic logic [69:0] g(input logic [69:0] a); return a; endfunction endmodule
class C; logic [69:0] f; logic [69:0] k;
  function new(input logic [69:0] a); k = a; endfunction
  function void set(input logic [69:0] a); f = a; endfunction
endclass
module t; sub u(); C o; logic [69:0] r1, r2, r3;
  task automatic tk(input logic [69:0] a, output logic [69:0] y); y = a; endtask
  initial begin o = new('1); o.set('z); r1 = u.g('z); tk('1, r2);
    #1 $display("A k=%h f=%h r1=%h r2=%h", o.k, o.f, r1, r2); o.set('1); r3 = o.f; o.f = 'z; #1 $display("B r3=%h f=%h", r3, o.f); end
  initial #100 $finish;
endmodule
