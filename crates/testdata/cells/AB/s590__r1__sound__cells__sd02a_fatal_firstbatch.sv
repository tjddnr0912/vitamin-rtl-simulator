module top;
  logic a;
  wire y;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  assign y = f(a);
  initial begin a = 1'b0; $fatal(1, "TB fatal"); end
  final $display("final y=%b", y);
endmodule
