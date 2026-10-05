interface ifc ();
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  parameter real R = f(2);
endinterface
module top;
  ifc i();
  initial begin #1 $display("R=%0.2f", i.R); $finish; end
endmodule
