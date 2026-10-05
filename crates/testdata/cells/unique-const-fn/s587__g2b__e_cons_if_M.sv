module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  class C;
    rand int x;
    constraint c { x == f(2); }
  endclass
  C c = new;
  initial begin #1 void'(c.randomize()); $display("x=%0d", c.x); $finish; end
endmodule
