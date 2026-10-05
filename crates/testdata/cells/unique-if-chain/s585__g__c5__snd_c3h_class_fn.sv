module top;
  class C;
    function void m(input logic x, input logic z);
      if (x) $display("t=%0t m-x", $time); else unique if (z) $display("t=%0t m-z", $time);
    endfunction
    function integer f(input logic x, input logic z);
      f = 0;
      if (x) f = 1; else unique if (z) f = 2;
    endfunction
  endclass
  function automatic void mv(input logic x, input logic z);
    if (x) $display("t=%0t m-x", $time); else unique if (z) $display("t=%0t m-z", $time);
  endfunction
  C o; integer r;
  initial begin
    o = new;
    #1 o.m(0, 0);
    #1 r = o.f(0, 0); $display("t=%0t r=%0d", $time, r);
    #1 o.m(0, 1);
    #1 mv(0, 0);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
