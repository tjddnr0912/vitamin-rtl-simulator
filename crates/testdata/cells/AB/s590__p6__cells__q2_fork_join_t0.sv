module top;
  logic [1:0] a, b;
  wire [1:0] y;
  function logic [1:0] f(input logic [1:0] x, input logic [1:0] z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return x ^ z;
  endfunction
  assign y = f(a, b);
  initial begin
    fork
      a = 2'b01;
      b = 2'b11;
    join
    $display("i0 y=%b", y);
    #1 $display("i1 y=%b", y);
    $finish;
  end
  initial #100 $finish;
endmodule
