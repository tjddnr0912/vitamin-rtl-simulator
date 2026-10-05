`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  class C;
    function logic [3:0] g(int n); $display("g(%0d)", n); return n[3:0]; endfunction
    function int meth(int n);
      case (g(n)) inside 4'd1: return 1; [4'd5:4'd6]: return 2; 4'd3: return 4; default: return 0; endcase
    endfunction
  endclass
  initial begin
    C c; c = new;
    $display("meth m=%0d", c.meth(3));
    $finish;
  end
endmodule
