module tb; typedef struct packed { logic a; logic [4:0] b; } st_t;
  localparam st_t A[2] = '{'{a: 1'b1, b: 5'd3}, '{1'b0, 5'd7}};
  initial begin $display("DIGEST=%0d", A[0].b); #1 $finish; end endmodule