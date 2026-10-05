interface ifc;
  function automatic int f(input int a);
    unique if (a == 1) return 1;
    return 7;
  endfunction
  logic [f(2):0] s;
endinterface
module top;
  function automatic int f(input int a); return 3; endfunction
  ifc i();
  localparam int P = f(2);
  logic [f(2):0] w;
  initial begin $display("P=%0d bw=%0d bs=%0d", P, $bits(w), $bits(i.s)); #1 $finish; end
endmodule
