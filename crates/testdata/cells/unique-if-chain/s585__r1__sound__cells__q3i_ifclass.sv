interface class IC;
  pure virtual function logic [1:0] f(input logic x, input logic z);
endclass
class C implements IC;
  task t(input logic x, input logic z); logic [1:0] r; unique if (x) r = 1; else if (z) r = 2; endtask
  virtual function logic [1:0] f(input logic x, input logic z); return {x, z}; endfunction
endclass
module top;
  logic a, b; logic [1:0] y; C obj;
  initial obj = new;
  assign y = obj.f(a, b);
  initial begin a = 0; b = 1; #1 b = 0; #1 $finish; end
endmodule
