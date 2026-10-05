module child;
  function automatic int cf(input int a);
    cf = 5;
    if (a == 1) cf = 10;
  endfunction
  localparam int P = cf(2);
  logic [P:0] v = '1;
endmodule
module top;
  child u();
  initial begin #1 $display("uP=%0d bv=%0d", u.P, $bits(u.v)); $finish; end
endmodule
