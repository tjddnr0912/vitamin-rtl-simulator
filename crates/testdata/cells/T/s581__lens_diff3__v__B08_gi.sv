module top;
  typedef struct packed {logic [39:0] a; logic [3:0] b;} st;
  localparam st SS = '{a: 40'h10_0000_000C, b: 4'hC};
  if (SS.a ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
