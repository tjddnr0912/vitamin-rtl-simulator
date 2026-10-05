module tb; typedef struct packed { logic a; logic [4:0] b; } st_t;
  if (1) begin : g
    localparam st_t P = '{a: 1'b1, b: 5'd3}; localparam W = P + 1;
    initial begin $display("DIGEST=%b %0d %0d %0d", P, $bits(P), P.b, W); #1 $finish; end
  end
endmodule