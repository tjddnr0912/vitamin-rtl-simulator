module top;
  logic a, b; logic [1:0] y;
  function void fv(input logic x, input logic z, output logic [1:0] r);
    r = 0;
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
  always_comb fv(a, b, y);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 b = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
endmodule
