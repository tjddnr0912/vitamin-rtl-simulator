module t;
  typedef struct packed { logic f; logic [15:0] bus; } e_t;
  e_t e; logic [3:0] v; logic [6:0] w;
  localparam int A = $bits({v, w});
  localparam int B = $bits({e.f, e.bus});
  initial begin #1 $display("A A=%0d B=%0d", A, B); $finish; end
endmodule
