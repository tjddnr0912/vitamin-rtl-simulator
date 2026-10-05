interface ifc #(parameter int W = g(2));
  function automatic int g(input int a); return 3; endfunction
  logic [W-1:0] s;
endinterface
module top;
  function automatic int g(input int a);
    unique if (a == 1) return 1;
    return 7;
  endfunction
  ifc i[1:0]();
  initial begin $display("b=%0d", $bits(i[1].s)); #1 $finish; end
endmodule
