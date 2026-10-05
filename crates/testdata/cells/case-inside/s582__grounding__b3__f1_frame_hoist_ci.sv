module top;
  int m;
  function automatic logic [3:0] g(input int n); $display("g(%0d)", n); return n[3:0]; endfunction
  function automatic int fn(input int n);
    case (g(n)) inside 4'd1: fn = 1; [4'd5:4'd6]: fn = 2; 4'b1?00: fn = 3; 4'd3: fn = 4; default: fn = 0; endcase
  endfunction
  task automatic tk(input int n, output int o);
    case (g(n)) inside 4'd1: o = 1; [4'd5:4'd6]: o = 2; 4'b1?00: o = 3; 4'd3: o = 4; default: o = 0; endcase
  endtask
  class C;
    function int meth(int n);
      case (g(n)) inside 4'd1: return 1; [4'd5:4'd6]: return 2; 4'b1?00: return 3; 4'd3: return 4; default: return 0; endcase
    endfunction
  endclass
  initial begin
    C c; c = new;
    m = fn(3); $display("fn m=%0d", m);
    tk(3, m); $display("tk m=%0d", m);
    m = c.meth(3); $display("meth m=%0d", m);
    #10 $finish;
  end
endmodule
