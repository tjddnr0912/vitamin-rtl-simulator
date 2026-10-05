class C;
  static function int f(input int a);
    int r; r = 0;
    r = r;
    return r + 10;
  endfunction
endclass
module top;
  localparam int P = C::f(3);
  initial begin $display("P=%0d", P); #1 $finish; end
  initial #100 $finish;
endmodule
