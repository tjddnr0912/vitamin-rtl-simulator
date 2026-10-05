module top;
  wire [1:0] y = f(1'b0, 1'b1);
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  initial begin
    $display("i0 y=%b", y);
    #0 $display("i1 y=%b", y);
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
