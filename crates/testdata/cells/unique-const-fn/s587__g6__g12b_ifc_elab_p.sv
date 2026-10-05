interface ifc ();
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  $info("el=%0d", f(2));
endinterface
module top;
  ifc i();
  initial begin #1 $finish; end
endmodule
