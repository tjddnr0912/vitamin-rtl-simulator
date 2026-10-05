class C;
  static function int f(input int a);
    int r; r = 0;
    if (a == 1) r = 1; else unique if (a == 2) r = 2;
    return r + 10;
  endfunction
endclass
module top;
  localparam int P = C::f(3);
  initial begin $display("P=%0d", P); #1 $finish; end
  initial #100 $finish;
endmodule
