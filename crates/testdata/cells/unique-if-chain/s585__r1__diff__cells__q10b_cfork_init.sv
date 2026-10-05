class C;
  task t(input logic x, input logic z);
    unique if (x) $display("t x t=%0t", $time);
    else if (z) $display("t z t=%0t", $time);
  endtask
  function logic [1:0] f(input logic x, input logic z);
    fork
      t(x, z);
    join_none
    return {x, z};
  endfunction
endclass
module top;
  logic a, b; logic [1:0] y;
  C obj;
  initial begin
    obj = new;
    a = 0; b = 1;
    y = obj.f(a, b);
    #1 b = 0;
    y = obj.f(a, b);
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
