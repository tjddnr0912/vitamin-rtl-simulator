interface ifc;
  logic s;
  function void ifv(input logic x, input logic z);
    unique if (x) s = 1;
    else if (z) s = 0;
  endfunction
  task it(input logic x, input logic z);
    unique if (x) s = 1;
    else if (z) s = 0;
  endtask
endinterface
class C;
  virtual ifc vif;
  function new(virtual ifc v); vif = v; endfunction
  task go(input logic x, input logic z);
    vif.ifv(x, z);
    vif.it(x, z);
  endtask
endclass
module top;
  ifc u_if();
  C obj;
  logic a = 0, b = 1;
  initial begin
    obj = new(u_if);
    #1 obj.go(a, b);
    #1 b = 0;
    obj.go(a, b);
    #1 $display("t=%0t end", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
