module top;
  logic a, b; logic [1:0] y;
  task t(input logic x);
    $display("t t=%0t x=%b", $time, x);
  endtask
  function logic [1:0] f(input logic x, input logic z);
    t(x);
    return {x, z};
  endfunction
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
