class P #(int W = 2);
  logic [W-1:0] r;
  task t(input logic x, input logic z); unique if (x) r = 1; else if (z) r = 2; endtask
  function logic [W-1:0] f(input logic x, input logic z); this.t(x, z); return {x, z}; endfunction
endclass
module top;
  logic a, b; logic [1:0] y; P #(2) obj;
  initial obj = new;
  assign y = obj.f(a, b);
  initial begin a = 0; b = 1; #1 $display("t=%0t y=%0d", $time, y); #1 b = 0; #1 $display("t=%0t y=%0d", $time, y); #1 $finish; end
endmodule
