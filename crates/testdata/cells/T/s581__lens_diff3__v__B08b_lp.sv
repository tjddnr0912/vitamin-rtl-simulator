module top;
  typedef struct packed {logic [39:0] a; logic [3:0] b;} st;
  localparam st SS = '{a: 40'h10_0000_000C, b: 4'hC};
  localparam R = (SS.b ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
