interface ifc #(parameter int P = fi(2)) ();
  function automatic int fi(input int a);
    fi = 6;
    if (a == 1) fi = 10;
  endfunction
  logic [P:0] d;
  localparam int Q = fi(3) + 1;
endinterface
module top;
  ifc i0();
  ifc #(.P(2)) i1();
  ifc ia[1:0] ();
  initial begin #1 $display("i0P=%0d i0b=%0d i0Q=%0d i1P=%0d i1b=%0d iab=%0d", i0.P, $bits(i0.d), i0.Q, i1.P, $bits(i1.d), $bits(ia[1].d)); $finish; end
endmodule
