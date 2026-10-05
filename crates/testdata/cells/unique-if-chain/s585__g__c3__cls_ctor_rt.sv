class C;
  int r;
  function new(input int x);
    r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endfunction
endclass
module top;
  C c;
  initial begin #1 c = new(0); $display("t=%0t r=%0d", $time, c.r); #1 $finish; end
endmodule
