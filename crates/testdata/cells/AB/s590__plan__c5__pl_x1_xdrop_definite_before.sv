module top;
  logic a; wire w; wire v;
  function logic f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  assign w = f(a);
  assign v = (w === 1'bz) ? 1'b0 : w;
  always @(v) $display("V t=%0t v=%b", $time, v);
  always @(posedge v) $display("PV t=%0t v=%b", $time, v);
  always @(negedge v) $display("NV t=%0t v=%b", $time, v);
  initial begin
    $display("i0 w=%b v=%b", w, v);
    #1 $display("t=%0t w=%b v=%b", $time, w, v);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
