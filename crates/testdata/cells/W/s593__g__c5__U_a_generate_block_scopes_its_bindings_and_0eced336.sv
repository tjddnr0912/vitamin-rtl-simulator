module tb;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } sa_t;
  typedef struct packed { logic [11:0] a; logic [3:0] b; } sb_t;
  sa_t S;
  if (1) begin : g
    localparam sb_t S = '{12'h111, 4'h2};
    initial $display("G=%h %h", S.a, S.b);
  end
  initial begin S = 8'h34; #1 $display("DIGEST=%h %h", S.a, S.b); #1 $finish; end
endmodule