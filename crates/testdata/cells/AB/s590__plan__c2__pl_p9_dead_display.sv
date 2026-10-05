module top;
  logic a, b; logic [1:0] y; wire [1:0] w;
  function logic [1:0] f(input logic x, input logic z);
    if (1'b0) $display("never");
    return {x, z};
  endfunction
  assign y = f(a, b);
  assign w = f(b, a);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  always @(posedge w[0]) $display("PW t=%0t w=%b", $time, w);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b w=%b", $time, y, w);
    b = 0;
    #1 $display("t=%0t y=%b w=%b", $time, y, w);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
