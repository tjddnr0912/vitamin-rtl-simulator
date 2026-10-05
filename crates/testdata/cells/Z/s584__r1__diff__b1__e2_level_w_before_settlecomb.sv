// Level always @(w) (settle wake) declared before a settle-woken comb; reads y but is not sensitive to it
module top;
  wire [1:0] w = 2'd1;
  logic [1:0] y;
  always @(w) $display("L t=%0t w=%b y=%b", $time, w, y);
  always_comb y = w + 2'd1;
  initial $display("I t=%0t y=%b", $time, y);
  initial #5 $display("E t=%0t y=%b", $time, y);
  initial #10 $finish;
endmodule
