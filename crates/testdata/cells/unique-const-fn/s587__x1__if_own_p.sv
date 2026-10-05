interface ifc;
  function automatic int f(input int a); return 3; endfunction
  logic [f(2):0] s;
  localparam int L = f(2);
endinterface
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  ifc i();
  initial begin #1 $display("bs=%0d L=%0d", $bits(i.s), i.L); $finish; end
endmodule
