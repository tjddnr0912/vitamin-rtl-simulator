module top;
  logic a, b; logic [1:0] y; wire [1:0] w;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b", $time, x);
    if (x) return 2'b10; else return 2'b01;
  endfunction
  assign y = f(a, b);
  assign w = f(a, b);
  initial begin
    $dumpfile("v14.vcd"); $dumpvars(0, top);
    a = 1; b = 0;
    #1 a = 0;
    #1 $finish;
  end
endmodule
