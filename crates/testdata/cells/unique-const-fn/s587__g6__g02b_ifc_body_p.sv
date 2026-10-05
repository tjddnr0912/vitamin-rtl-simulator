interface ifc ();
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  parameter int P = f(2);
endinterface
module top;
  ifc i();
  initial begin #1 $display("P=%0d", i.P); $finish; end
endmodule
