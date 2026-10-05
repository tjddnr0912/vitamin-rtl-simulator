class C;
  static function int f(input int x);
    int r; r = 7;
    if (x == 1) r = 1; else unique if (x == 2) r = 2;
    return r;
  endfunction
endclass
module top;
  localparam int P = C::f(0);
  initial begin $display("P=%0d", P); #1 $finish; end
endmodule
