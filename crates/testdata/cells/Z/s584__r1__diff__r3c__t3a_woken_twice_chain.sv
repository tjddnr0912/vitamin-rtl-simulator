module top;
  logic [1:0] src = 2'd1;
  logic en;
  logic [1:0] p, m, y;
  initial en = 1'b1;
  always_comb p = src;
  always_comb begin m = en ? p : 2'd0; $display("M t=%0t p=%b en=%b", $time, p, en); end
  always_comb begin unique case (m) 2'd1: y = 2'd1; 2'd2: y = 2'd2; endcase end
  initial #1 $display("t=%0t y=%0d", $time, y);
  initial #5 $finish;
endmodule
