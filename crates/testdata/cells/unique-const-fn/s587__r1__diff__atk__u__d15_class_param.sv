package pk;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
endpackage
module top;
  class C #(parameter int P = pk::f(2));
    static function int get();
      return P;
    endfunction
  endclass
  initial begin #1 $display("C=%0d", C#()::get()); $finish; end
endmodule
