module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    if (x) return 2'b10; else return 2'b01;
  endfunction
  assign y = f(a, b);
  initial begin
    $dumpfile("v13.vcd"); $dumpvars(0, top);
    a = 1; b = 0;
    #1 a = 0;
    #1 $finish;
  end
endmodule
