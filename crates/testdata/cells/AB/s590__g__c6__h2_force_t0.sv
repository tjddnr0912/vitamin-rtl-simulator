module top;
  logic a, b; wire [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y = f(a, b);
  initial begin
    force y = 2'b11;
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    release y;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
