module top;
  supply1 vdd;
  function automatic logic f(input logic x);
    $display("f x=%b t=%0t", x, $time);
    f = x;
  endfunction
  wire y = f(vdd);
  initial $display("init y=%b", y);
  always @(posedge y) $display("pos y t=%0t", $time);
  initial #3 $finish;
endmodule
