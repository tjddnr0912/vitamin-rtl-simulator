class P #(parameter W = 8); logic [W-1:0] f = 'x; logic [W-1:0] g; function new(); g = 'z; endfunction endclass
class B; logic [69:0] bf; endclass
class D extends B; logic [69:0] df; function new(); bf = 'x; df = 'z; endfunction endclass
typedef struct packed { logic [39:0] a; logic [29:0] b; } s_t;
module t; P #(70) op; D od;
  initial begin op = new; od = new;
    #1 $display("A pf=%h pg=%h bf=%h df=%h", op.f, op.g, od.bf, od.df); end
  initial #100 $finish;
endmodule
