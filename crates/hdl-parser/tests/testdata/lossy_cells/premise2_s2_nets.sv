interface nif; wire [69:0] n; endinterface
module drv(nif p); assign p.n = 'z; endmodule
module t;
  typedef struct packed { logic [39:0] a; logic [29:0] b; } s_t;
  wire [1:0][34:0] nm = 'x; nif ni(); drv d(ni);
  logic [69:0] b8; int i;
  initial begin i = 3; b8 = '0; b8[i +: 35] = 'z; #1 $display("A nm=%h pn=%h b8=%h", nm, ni.n, b8); end
  initial #100 $finish;
endmodule
