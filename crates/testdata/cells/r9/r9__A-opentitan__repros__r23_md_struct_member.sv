module t;
  typedef struct packed { logic c; logic [2:0] v; } f_t;
  f_t [1:0][2:0] a;
  logic [2:0] o;
  always_comb for (int k = 0; k < 3; k++) o[k] = a[1][k].c;
  initial begin a = 24'h8_0_8_0_8_0 | 24'h800000; #1 $display("A o=%b", o); $finish; end
endmodule
