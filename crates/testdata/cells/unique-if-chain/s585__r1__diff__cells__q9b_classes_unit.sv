  class P;
    function int pf(input logic x, input logic z);
      unique if (x) return 1;
      else if (z) return 2;
      return 0;
    endfunction
    task pt(input logic x, input logic z);
      unique if (x) $display("pt x");
      else if (z) $display("pt z");
    endtask
  endclass
class B;
  virtual task vt(input logic x, input logic z);
    unique if (x) $display("B x");
    else if (z) $display("B z");
  endtask
endclass
class D extends B;
  virtual task vt(input logic x, input logic z);
    unique if (x) $display("D x");
    else if (z) $display("D z");
  endtask
endclass
class Q #(int N = 1);
  task qt(input logic x, input logic z);
    unique if (x) $display("Q x %0d", N);
    else if (z) $display("Q z %0d", N);
  endtask
endclass
module top;
  logic a = 0, b = 0;
  int v;
  P p; B bb; D d; Q #(2) q;
  initial begin
    p = new; d = new; bb = d; q = new;
    #1 v = p.pf(a, b);
    #1 p.pt(a, b);
    #1 bb.vt(a, b);
    #1 q.qt(a, b);
    #1 $display("t=%0t end v=%0d", $time, v);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
