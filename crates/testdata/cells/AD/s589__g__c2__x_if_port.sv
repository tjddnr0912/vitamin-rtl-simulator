interface ifc;
  function automatic int f(input int a); return 3; endfunction
  logic [f(2):0] s;
endinterface
module c (ifc i);
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  initial #1 $display("bs=%0d", $bits(i.s));
endmodule
module top;
  ifc i();
  c u (.i(i));
  initial #2 $finish;
endmodule
