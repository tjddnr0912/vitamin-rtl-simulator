module top;
  logic a = 0, b = 0;
  logic [1:0] r = 0;
  int q[3] = '{1, 2, 3};
  initial begin
    #1 foreach (q[k]) begin unique if (q[k] == 2) r = 1; else if (a) r = 2; $display("fe k=%0d", k); end
    #1 foreach (q[k]) begin unique if (q[k] == 2) continue; else if (a) r = 2; $display("fe2 k=%0d", k); end
    #1 $finish;
  end
  initial #100 $finish;
endmodule
