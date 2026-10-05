`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  function int f(); return X; endfunction
  function bit g(); return X < 0; endfunction
  initial $display("f=%0d g=%0d", f(), g());
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
