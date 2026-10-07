module t;
  typedef struct packed { logic [7:0] a; logic [3:0] s; } h_t;
  localparam int P = 6;
  localparam int K = 2;
  h_t h;
  always_comb begin
    h = '0;
    h.a[P-1:0] = 6'h2d;
    h.s[K] = 1'b1;
  end
  initial begin #1 $display("A h=%h", h); $finish; end
endmodule
