module top;
  logic a = 1'b0;
  wire y1, y2;
  function automatic logic f1(input logic x); if (x == 1'b0) $fatal(1, "F1"); return x; endfunction
  function automatic logic f2(input logic x); $display("f2 t=%0t x=%b", $time, x); return x; endfunction
  assign y1 = f1(1'b0);
  assign y2 = f2(a);
endmodule
