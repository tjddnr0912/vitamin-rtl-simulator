module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  class C #(parameter int P = f(1));
    static function int get(); return P; endfunction
  endclass
  initial begin #1 $display("P=%0d", C#()::get()); $finish; end
endmodule
