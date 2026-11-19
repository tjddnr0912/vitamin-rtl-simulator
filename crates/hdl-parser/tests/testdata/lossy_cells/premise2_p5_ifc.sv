interface ifc; logic [69:0] s1, s2, s3; logic [69:0] s5 = 'x;
  modport mp(output s2);
endinterface
module drv(ifc p, ifc.mp q); initial begin p.s1 = 'z; q.s2 = 'x; end endmodule
module t; ifc i(); drv d(i, i); virtual ifc vi;
  initial begin vi = i; vi.s3 = 'z; #1 $display("A s1=%h s2=%h s3=%h s5=%h", i.s1, i.s2, i.s3, i.s5); end
  initial #100 $finish;
endmodule
