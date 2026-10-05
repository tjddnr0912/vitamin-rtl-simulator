module top;
  int m;
  class C;
    function logic [3:0] g(int n); $display("g(%0d)", n); return n[3:0]; endfunction
    function int meth(int n);
      case (g(n)) 4'd1: return 1; 4'd5: return 2; 4'd12: return 3; 4'd3: return 4; default: return 0; endcase
    endfunction
  endclass
  initial begin
    C c; c = new;
    m = c.meth(3); $display("meth m=%0d", m);
    #10 $finish;
  end
endmodule
