interface ifc #(parameter int W = cf(2)) ();
  function automatic int cf(input int a);
    cf = 3;
    unique if (a == 1) cf = 10;
  endfunction
  logic [W:0] d;
endinterface
module top;
  function automatic int cf(input int a);
    cf = 7;
  endfunction
  ifc ia[1:0] ();
  initial begin #1 $display("b=%0d W=%0d", $bits(ia[1].d), ia[1].W); $finish; end
endmodule
