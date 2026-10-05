package pk;
  task pt(input logic x, input logic z, output logic [1:0] o); o = 0; if (x) o = 1; else unique if (z) o = 2; endtask
endpackage
class K;
  task m(input logic x, input logic z); if (x) $display("cx"); else unique if (z) $display("cz"); endtask
endclass
module top; import pk::*;
  logic a = 0, b = 0; logic [1:0] y;
  task st(input logic x, input logic z, output logic [1:0] o); o = 0; if (x) o = 1; else unique if (z) o = 2; endtask
  K k;
  initial begin
    k = new;
    #1 st(a, b, y);     $display("t=%0t static task done", $time);
    #1 pt(a, b, y);     $display("t=%0t pkg task done", $time);
    #1 k.m(a, b);       $display("t=%0t class task done", $time);
    #1 fork begin if (a) $display("cx"); else unique if (b) $display("cz"); end join $display("t=%0t fork child done", $time);
    #1 $finish;
  end
  final begin if (a) $display("cx"); else unique if (b) $display("cz"); end
endmodule
